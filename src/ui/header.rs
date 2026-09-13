use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use crate::config::{AppConfig, ThemeMode};
use crate::theme::Theme;
use crate::ui::gauge::format_duration;

pub fn render_header(
    buf: &mut Buffer,
    area: Rect,
    config: &AppConfig,
    theme: &Theme,
    uptime_secs: u64,
) {
    if area.height < 1 || area.width < 10 {
        return;
    }

    let y = area.y;
    let mut x = area.x;

    // Logo: WTOP in bold neon
    let logo_style = Style::default().fg(theme.title_focused).add_modifier(Modifier::BOLD);
    let logo = " ⯈ wtop ";
    for ch in logo.chars() {
        if x < area.right() {
            buf[(x, y)].set_char(ch).set_style(logo_style);
            x += 1;
        }
    }

    // Uptime
    let uptime_str = format!("up {}", format_duration(uptime_secs));
    let uptime_style = Style::default().fg(theme.text_dim);
    for ch in uptime_str.chars() {
        if x < area.right() {
            buf[(x, y)].set_char(ch).set_style(uptime_style);
            x += 1;
        }
    }

    // Right-aligned badges: Theme, Rate, Help, Quit
    let theme_badge = match config.theme {
        ThemeMode::Dark => "[t: DARK]",
        ThemeMode::Light => "[t: LIGHT]",
    };
    let rate_badge = format!("[+/-: {}ms]", config.sampling_rate_ms);
    let help_badge = "[?: Help]";
    let quit_badge = "[q: Quit]";

    let badges = format!("{}  {}  {}  {} ", theme_badge, rate_badge, help_badge, quit_badge);
    let badges_len = badges.chars().count() as u16;

    if area.width > badges_len + 20 {
        let start_x = area.right().saturating_sub(badges_len);
        let mut cur_x = start_x;
        for ch in badges.chars() {
            let style = if ch == '[' || ch == ']' {
                Style::default().fg(theme.border_normal)
            } else if ch == '?' || ch == 'q' || ch == 't' || ch == '+' || ch == '-' {
                Style::default().fg(theme.title_focused).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_main)
            };
            if cur_x < area.right() {
                buf[(cur_x, y)].set_char(ch).set_style(style);
                cur_x += 1;
            }
        }
    }
}
