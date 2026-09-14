use crate::config::{AppConfig, ThemeMode};
use crate::theme::Theme;
use crate::ui::gauge::format_duration;
use crate::ui::text::draw_str_with;
use crate::updater::UpdateStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

pub fn render_header(
    buf: &mut Buffer,
    area: Rect,
    config: &AppConfig,
    theme: &Theme,
    uptime_secs: u64,
    update_status: &UpdateStatus,
) {
    if area.height < 1 || area.width < 10 {
        return;
    }

    let y = area.y;
    let right = area.right();

    // Logo: WTOP in bold neon
    let logo_style = Style::default()
        .fg(theme.title_focused)
        .add_modifier(Modifier::BOLD);
    let x = draw_str_with(buf, area.x, y, right, " ⯈ wtop ", |_| logo_style);

    // Uptime
    let uptime_str = format!("up {}", format_duration(uptime_secs));
    let uptime_style = Style::default().fg(theme.text_dim);
    draw_str_with(buf, x, y, right, &uptime_str, |_| uptime_style);

    // Right-aligned badges: Theme, Rate, Update (if any), Help, Quit
    let theme_badge = match config.theme {
        ThemeMode::Dark => "[t: DARK]",
        ThemeMode::Light => "[t: LIGHT]",
    };
    let rate_badge = format!("[+/-: {}ms]", config.sampling_rate_ms);
    // Idle/Error render no badge at all -- this check is silent on failure,
    // never nagging the user about network problems.
    let update_badge = match update_status {
        UpdateStatus::Available(info) => Some(format!("[u: Update v{} available]", info.version)),
        UpdateStatus::Downloading => Some("[u: Downloading update...]".to_string()),
        UpdateStatus::Ready => Some("[u: Relaunching...]".to_string()),
        UpdateStatus::Idle | UpdateStatus::Error(_) => None,
    };
    let github_badge = "[g: GitHub]";
    let help_badge = "[?: Help]";
    let quit_badge = "[q: Quit]";

    let mut badges = format!("{}  {}  ", theme_badge, rate_badge);
    if let Some(update_badge) = &update_badge {
        badges.push_str(update_badge);
        badges.push_str("  ");
    }
    badges.push_str(&format!(
        "{}  {}  {} ",
        github_badge, help_badge, quit_badge
    ));
    let badges_len = badges.chars().count() as u16;

    if area.width > badges_len + 20 {
        let start_x = right.saturating_sub(badges_len);
        draw_str_with(buf, start_x, y, right, &badges, |ch| {
            if ch == '[' || ch == ']' {
                Style::default().fg(theme.border_normal)
            } else if ch == '?' || ch == 'q' || ch == 't' || ch == 'u' || ch == 'g' || ch == '+' || ch == '-' {
                Style::default()
                    .fg(theme.title_focused)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_main)
            }
        });
    }
}
