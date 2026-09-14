use crate::model::NetworkData;
use crate::theme::Theme;
use crate::ui::braille::{StackedSeries, render_stacked_charts};
use crate::ui::gauge::{format_bytes, format_speed};
use crate::ui::text::{draw_str, truncate};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// Chars taken by the fixed `" NETWORK:  "` title frame around the variable
/// -length adapter name (mirrors `cpu_panel`'s `TITLE_FRAME_LEN`).
const TITLE_FRAME_LEN: usize = 11;

/// Width of the vertical download/upload readout column.
const STATS_COL_WIDTH: u16 = 20;

pub fn render_net_panel(
    buf: &mut Buffer,
    area: Rect,
    net: &NetworkData,
    theme: &Theme,
    focused: bool,
) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let title = net_title(&net.active_iface, area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_border_style(focused))
        .title(title)
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 2 {
        return;
    }

    // Left: vertical download/upload readout (the numbers, always shown).
    // Right: the braille chart. Below a minimum width the chart is the part
    // that gets dropped first, never the throughput numbers.
    let stats_w = STATS_COL_WIDTH.min(inner.width);
    let show_chart = inner.width > stats_w + 8;

    let stats_area = Rect {
        x: inner.x,
        y: inner.y,
        width: if show_chart { stats_w } else { inner.width },
        height: inner.height,
    };
    render_speed_stats(buf, stats_area, net, theme);

    if show_chart {
        let chart_area = Rect {
            x: inner.x + stats_w + 1,
            y: inner.y,
            width: inner.width - stats_w - 1,
            height: inner.height,
        };
        render_stacked_charts(
            buf,
            chart_area,
            StackedSeries {
                data: &net.rx_history,
                low: theme.border_normal,
                high: theme.net_rx,
            },
            StackedSeries {
                data: &net.tx_history,
                low: theme.border_normal,
                high: theme.net_tx,
            },
        );
    }
}

/// Draws the download/upload speeds stacked vertically (one direction above
/// the other), tiering down the detail shown as height shrinks but never
/// dropping the current speed numbers themselves.
fn render_speed_stats(buf: &mut Buffer, area: Rect, net: &NetworkData, theme: &Theme) {
    if area.height == 0 {
        return;
    }

    let down_speed = format_speed(net.rx_bytes_sec);
    let up_speed = format_speed(net.tx_bytes_sec);

    let rx_style = Style::default()
        .fg(theme.net_rx)
        .add_modifier(Modifier::BOLD);
    let tx_style = Style::default()
        .fg(theme.net_tx)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.text_dim);
    let main = Style::default().fg(theme.text_main);

    let lines: Vec<(String, Style)> = if area.height >= 6 {
        vec![
            ("▼ DOWNLOAD".to_string(), rx_style),
            (format!(" {}", down_speed), main),
            (format!(" Tot {}", format_bytes(net.rx_total_bytes)), dim),
            ("▲ UPLOAD".to_string(), tx_style),
            (format!(" {}", up_speed), main),
            (format!(" Tot {}", format_bytes(net.tx_total_bytes)), dim),
        ]
    } else if area.height >= 2 {
        vec![
            (format!("▼ {}", down_speed), rx_style),
            (format!("▲ {}", up_speed), tx_style),
        ]
    } else {
        vec![(format!("▼{} ▲{}", down_speed, up_speed), main)]
    };

    for (i, (text, style)) in lines.iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.bottom() {
            break;
        }
        draw_str(buf, area.x, y, area.right(), text, *style);
    }
}

/// Builds the network panel's block title from the active interface name,
/// truncating with an ellipsis rather than letting ratatui silently clip it.
/// Falls back to a bare " NETWORK " title when there's no name or no room.
fn net_title(iface: &str, area_width: u16) -> String {
    if iface.is_empty() {
        return " NETWORK ".to_string();
    }
    let avail = area_width.saturating_sub(2) as usize; // inside the left/right border
    let name_budget = avail.saturating_sub(TITLE_FRAME_LEN);
    if name_budget < 4 {
        return " NETWORK ".to_string();
    }
    format!(" NETWORK: {} ", truncate(iface, name_budget))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for the reported bug (now against the split panel):
    /// even on a panel too narrow to also show the braille chart, the
    /// download/upload speed numbers must always be drawn somewhere.
    #[test]
    fn net_panel_always_shows_speed_numbers_on_narrow_panel() {
        let theme = Theme::dark();
        let net = NetworkData {
            rx_bytes_sec: 512_000.0,
            tx_bytes_sec: 102_400.0,
            rx_total_bytes: 2 * 1024 * 1024 * 1024,
            tx_total_bytes: 512 * 1024 * 1024,
            active_iface: "Ethernet0 Long Adapter Name".to_string(),
            ..Default::default()
        };

        // Narrow enough that the chart must be dropped (stats_w + 8 is the
        // show_chart threshold), but still wide enough to render the panel.
        let area = Rect::new(0, 0, 18, 10);
        let mut buf = Buffer::empty(area);

        render_net_panel(&mut buf, area, &net, &theme, false);

        let rendered: String = (area.y..area.bottom())
            .map(|y| {
                (area.x..area.right())
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            rendered.contains(&format_speed(net.rx_bytes_sec)),
            "missing RX speed: {rendered:?}"
        );
        assert!(
            rendered.contains(&format_speed(net.tx_bytes_sec)),
            "missing TX speed: {rendered:?}"
        );
    }

    #[test]
    fn net_title_truncates_long_iface_name() {
        let title = net_title("Ethernet0 Long Adapter Name", 20);
        assert!(title.starts_with(" NETWORK: "));
        assert!(title.chars().count() as u16 <= 20);
    }

    #[test]
    fn net_title_falls_back_when_too_narrow() {
        assert_eq!(net_title("Wi-Fi", 8), " NETWORK ");
    }
}
