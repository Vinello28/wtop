use crate::model::{GpuData, NetworkData};
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::{format_bytes, format_speed};
use crate::ui::text::{draw_str, draw_str_with, truncate};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_net_gpu_panel(
    buf: &mut Buffer,
    area: Rect,
    gpu: &GpuData,
    net: &NetworkData,
    theme: &Theme,
    focused: bool,
) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_border_style(focused))
        .title(" GPU & NETWORK ")
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 3 {
        return;
    }

    // Split vertically: Top for GPU, Bottom for Network
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(inner);

    let gpu_area = sections[0];
    let net_area = sections[1];

    // --- GPU SECTION ---
    if gpu_area.height >= 2 {
        let vram_str = if gpu.dedicated_vram_used > 0 {
            format!("VRAM: {}", format_bytes(gpu.dedicated_vram_used))
        } else {
            String::new()
        };

        // Reserve space for the fixed percentage/VRAM part first, then give
        // whatever remains to the (potentially long) adapter name — never
        // truncate the composed line blindly from the end.
        let fixed_part = format!("GPU: {:5.1}%  {}  ", gpu.utilization_pct, vram_str);
        let name_budget = (gpu_area.width as usize).saturating_sub(fixed_part.chars().count());
        let gpu_header = format!("{}{}", fixed_part, truncate(&gpu.name, name_budget));

        draw_str(
            buf,
            gpu_area.x,
            gpu_area.y,
            gpu_area.right(),
            &gpu_header,
            Style::default()
                .fg(theme.gpu_primary)
                .add_modifier(Modifier::BOLD),
        );

        let chart_area = Rect {
            x: gpu_area.x,
            y: gpu_area.y + 1,
            width: gpu_area.width,
            height: gpu_area.height.saturating_sub(1),
        };

        if chart_area.height > 0 {
            let chart =
                BrailleChart::new(&gpu.history, 100.0, theme.gpu_primary, theme.gpu_secondary);
            chart.render(chart_area, buf);
        }
    }

    // --- NETWORK SECTION ---
    if net_area.height >= 2 {
        // The throughput numbers are the important, fixed-length part of
        // this line — reserve their width first. The interface name is
        // free-text and comes first visually, so on a narrow panel it must
        // be the part that shrinks (or disappears), never the numbers.
        let suffix = format!(
            "▼ {:>8} (Tot: {})  ▲ {:>8} (Tot: {})",
            format_speed(net.rx_bytes_sec),
            format_bytes(net.rx_total_bytes),
            format_speed(net.tx_bytes_sec),
            format_bytes(net.tx_total_bytes)
        );
        let reserved = "NET ".chars().count() + suffix.chars().count();
        let name_budget = (net_area.width as usize).saturating_sub(reserved);
        let iface_label = if net.active_iface.is_empty() || name_budget < 4 {
            String::new()
        } else {
            format!(
                "({}) ",
                truncate(&net.active_iface, name_budget.saturating_sub(3))
            )
        };

        let net_header = format!("NET {}{}", iface_label, suffix);

        draw_str_with(
            buf,
            net_area.x,
            net_area.y,
            net_area.right(),
            &net_header,
            |ch| {
                let color = if ch == '▼' {
                    theme.net_rx
                } else if ch == '▲' {
                    theme.net_tx
                } else {
                    theme.text_main
                };
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            },
        );

        let chart_area = Rect {
            x: net_area.x,
            y: net_area.y + 1,
            width: net_area.width,
            height: net_area.height.saturating_sub(1),
        };

        if chart_area.height > 0 {
            // Find dynamic max bandwidth for scaling chart
            let max_rx = net.rx_history.iter().copied().fold(1024.0, f64::max);
            let max_tx = net.tx_history.iter().copied().fold(1024.0, f64::max);
            let chart_max = max_rx.max(max_tx);

            // Render RX chart
            let chart = BrailleChart::new(&net.rx_history, chart_max, theme.net_rx, theme.net_tx);
            chart.render(chart_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for the reported bug: on a panel too narrow to show
    /// both a long interface name and the throughput numbers, the numbers
    /// must always win -- only the name shrinks or disappears.
    #[test]
    fn net_header_never_truncates_throughput_on_narrow_panel() {
        let theme = Theme::dark();
        let net = NetworkData {
            rx_bytes_sec: 512_000.0,
            tx_bytes_sec: 102_400.0,
            rx_total_bytes: 2 * 1024 * 1024 * 1024,
            tx_total_bytes: 512 * 1024 * 1024,
            active_iface: "Ethernet0 Long Adapter Name".to_string(),
            ..Default::default()
        };
        let gpu = GpuData::default();

        let suffix = format!(
            "▼ {:>8} (Tot: {})  ▲ {:>8} (Tot: {})",
            format_speed(net.rx_bytes_sec),
            format_bytes(net.rx_total_bytes),
            format_speed(net.tx_bytes_sec),
            format_bytes(net.tx_total_bytes),
        );
        let numbers_only_width = "NET ".chars().count() + suffix.chars().count();

        // Just wide enough for "NET " + the numbers, far too narrow to also
        // fit the interface name in front of them.
        let inner_width = (numbers_only_width + 3) as u16;
        let area = Rect::new(0, 0, inner_width + 2, 10);
        let mut buf = Buffer::empty(area);

        render_net_gpu_panel(&mut buf, area, &gpu, &net, &theme, false);

        let net_row_y = (area.y..area.bottom())
            .find(|&y| (area.x..area.right()).any(|x| buf[(x, y)].symbol() == "▼"))
            .expect("network header row not found");
        let row: String = (area.x..area.right())
            .map(|x| buf[(x, net_row_y)].symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            row.contains(&format_speed(net.rx_bytes_sec)),
            "row missing RX speed: {row:?}"
        );
        assert!(
            row.contains(&format_bytes(net.rx_total_bytes)),
            "row missing RX total: {row:?}"
        );
        assert!(
            row.contains(&format_speed(net.tx_bytes_sec)),
            "row missing TX speed: {row:?}"
        );
        assert!(
            row.contains(&format_bytes(net.tx_total_bytes)),
            "row missing TX total: {row:?}"
        );
        assert!(
            !row.contains("Ethernet0 Long Adapter Name"),
            "long iface name should have been dropped, not the numbers: {row:?}"
        );
    }
}
