use crate::model::BatteryData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::{format_duration, render_dual_bar};
use crate::ui::text::draw_str;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_battery_panel(
    buf: &mut Buffer,
    area: Rect,
    battery: &BatteryData,
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
        .title(" BATTERY ")
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 1 {
        return;
    }

    if !battery.is_present {
        draw_str(
            buf,
            inner.x,
            inner.y,
            inner.right(),
            "No battery detected",
            Style::default().fg(theme.text_dim),
        );
        return;
    }

    // Batteries are the inverse of every other gauge in the app: full (100%)
    // is good, empty (0%) is bad. Re-derive the color from the existing
    // low/mid/high gradient instead of adding dedicated theme fields.
    let color = theme.usage_color(100.0 - battery.percent);

    let pct_str = format!("{:5.1}%", battery.percent);
    let px = draw_str(
        buf,
        inner.x,
        inner.y,
        inner.right(),
        &pct_str,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    );

    let bar_x = px + 1;
    if bar_x < inner.right() {
        let bar_area = Rect {
            x: bar_x,
            y: inner.y,
            width: inner.right() - bar_x,
            height: 1,
        };
        render_dual_bar(buf, bar_area, battery.percent, color, theme.border_normal);
    }

    if inner.height >= 2 {
        let detail = status_line(battery);
        draw_str(
            buf,
            inner.x,
            inner.y + 1,
            inner.right(),
            &detail,
            Style::default().fg(theme.text_main),
        );
    }

    if inner.height >= 4 {
        let chart_area = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height - 2,
        };
        // Colors swapped vs. every other chart for the same reason as the
        // gauge above: low value = red (bad), high value = green (good).
        let chart = BrailleChart::new(&battery.history, 100.0, theme.cpu_high, theme.cpu_low);
        chart.render(chart_area, buf);
    }
}

/// Builds the status line under the percentage/gauge row: charge state plus,
/// when discharging and known, the remaining time.
fn status_line(battery: &BatteryData) -> String {
    if battery.charging {
        return "Charging".to_string();
    }
    if battery.ac_online {
        return "Plugged in".to_string();
    }
    match battery.seconds_remaining {
        Some(secs) => format!("On battery  {} left", format_duration(secs as u64)),
        None => "On battery".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_charging_takes_priority() {
        let battery = BatteryData {
            is_present: true,
            percent: 50.0,
            ac_online: true,
            charging: true,
            seconds_remaining: None,
            history: vec![],
        };
        assert_eq!(status_line(&battery), "Charging");
    }

    #[test]
    fn status_line_plugged_in_without_remaining_time() {
        let battery = BatteryData {
            is_present: true,
            percent: 100.0,
            ac_online: true,
            charging: false,
            seconds_remaining: None,
            history: vec![],
        };
        assert_eq!(status_line(&battery), "Plugged in");
    }

    #[test]
    fn status_line_on_battery_with_remaining_time() {
        let battery = BatteryData {
            is_present: true,
            percent: 42.0,
            ac_online: false,
            charging: false,
            seconds_remaining: Some(3665),
            history: vec![],
        };
        assert_eq!(status_line(&battery), "On battery  1h 01m 05s left");
    }

    #[test]
    fn status_line_on_battery_unknown_remaining_time() {
        let battery = BatteryData {
            is_present: true,
            percent: 42.0,
            ac_online: false,
            charging: false,
            seconds_remaining: None,
            history: vec![],
        };
        assert_eq!(status_line(&battery), "On battery");
    }

    #[test]
    fn no_battery_shows_placeholder_text() {
        let theme = Theme::dark();
        let battery = BatteryData::default();
        let area = Rect::new(0, 0, 30, 6);
        let mut buf = Buffer::empty(area);
        render_battery_panel(&mut buf, area, &battery, &theme, false);

        let rendered: String = (area.y..area.bottom())
            .map(|y| {
                (area.x..area.right())
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("No battery detected"));
    }
}
