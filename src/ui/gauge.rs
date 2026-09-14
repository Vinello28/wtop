use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

pub fn render_mini_bar(buf: &mut Buffer, area: Rect, pct: f64, theme: &Theme, empty_char: char) {
    if area.width < 1 || area.height < 1 {
        return;
    }

    let clamped = pct.clamp(0.0, 100.0);
    let color = theme.usage_color(clamped);
    let width = area.width as usize;

    let total_eighths = ((clamped / 100.0) * (width as f64 * 8.0)).round() as usize;
    let full_blocks = total_eighths / 8;
    let remainder = total_eighths % 8;

    let sub_blocks = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

    for i in 0..width {
        let x = area.x + i as u16;
        let y = area.y;

        if i < full_blocks {
            buf[(x, y)]
                .set_char('█')
                .set_style(Style::default().fg(color));
        } else if i == full_blocks && remainder > 0 {
            buf[(x, y)]
                .set_char(sub_blocks[remainder])
                .set_style(Style::default().fg(color));
        } else {
            buf[(x, y)]
                .set_char(empty_char)
                .set_style(Style::default().fg(theme.border_normal));
        }
    }
}

pub fn render_dual_bar(buf: &mut Buffer, area: Rect, pct: f64, color: Color, dim_color: Color) {
    if area.width < 1 || area.height < 1 {
        return;
    }

    let clamped = pct.clamp(0.0, 100.0);
    let width = area.width as usize;

    let total_eighths = ((clamped / 100.0) * (width as f64 * 8.0)).round() as usize;
    let full_blocks = total_eighths / 8;
    let remainder = total_eighths % 8;

    let sub_blocks = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

    for i in 0..width {
        let x = area.x + i as u16;
        let y = area.y;

        if i < full_blocks {
            buf[(x, y)]
                .set_char('█')
                .set_style(Style::default().fg(color));
        } else if i == full_blocks && remainder > 0 {
            buf[(x, y)]
                .set_char(sub_blocks[remainder])
                .set_style(Style::default().fg(color));
        } else {
            buf[(x, y)]
                .set_char('░')
                .set_style(Style::default().fg(dim_color));
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_speed(bytes_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_sec >= GB {
        format!("{:.2} GB/s", bytes_sec / GB)
    } else if bytes_sec >= MB {
        format!("{:.1} MB/s", bytes_sec / MB)
    } else if bytes_sec >= KB {
        format!("{:.0} KB/s", bytes_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_sec)
    }
}

pub fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;

    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, mins, s)
    } else {
        format!("{}m {:02}s", mins, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024 * 5), "5.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 16), "16.00 GB");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0.0), "0 B/s");
        assert_eq!(format_speed(1024.0 * 50.0), "50 KB/s");
        assert_eq!(format_speed(1024.0 * 1024.0 * 2.5), "2.5 MB/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "0m 45s");
        assert_eq!(format_duration(3665), "1h 01m 05s");
        assert_eq!(format_duration(90000), "1d 01h 00m");
    }
}
