use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, BorderType, Widget};
use crate::model::{GpuData, NetworkData};
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::{format_bytes, format_speed};

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

        let gpu_header = format!(
            "GPU: {:5.1}%  {}  {}",
            gpu.utilization_pct,
            vram_str,
            gpu.name
        );

        let mut gx = gpu_area.x;
        for ch in gpu_header.chars() {
            if gx < gpu_area.right() {
                buf[(gx, gpu_area.y)]
                    .set_char(ch)
                    .set_style(Style::default().fg(theme.gpu_primary).add_modifier(Modifier::BOLD));
                gx += 1;
            }
        }

        let chart_area = Rect {
            x: gpu_area.x,
            y: gpu_area.y + 1,
            width: gpu_area.width,
            height: gpu_area.height.saturating_sub(1),
        };

        if chart_area.height > 0 {
            let chart = BrailleChart::new(&gpu.history, 100.0, theme.gpu_primary, theme.gpu_secondary);
            chart.render(chart_area, buf);
        }
    }

    // --- NETWORK SECTION ---
    if net_area.height >= 2 {
        let iface_label = if net.active_iface.is_empty() {
            String::new()
        } else {
            format!("({}) ", net.active_iface)
        };

        let net_header = format!(
            "NET {} ▼ {:>8} (Tot: {})  ▲ {:>8} (Tot: {})",
            iface_label,
            format_speed(net.rx_bytes_sec),
            format_bytes(net.rx_total_bytes),
            format_speed(net.tx_bytes_sec),
            format_bytes(net.tx_total_bytes)
        );

        let mut nx = net_area.x;
        for ch in net_header.chars() {
            let color = if ch == '▼' {
                theme.net_rx
            } else if ch == '▲' {
                theme.net_tx
            } else {
                theme.text_main
            };
            if nx < net_area.right() {
                buf[(nx, net_area.y)]
                    .set_char(ch)
                    .set_style(Style::default().fg(color).add_modifier(Modifier::BOLD));
                nx += 1;
            }
        }

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
