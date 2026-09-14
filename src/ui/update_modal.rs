use crate::theme::Theme;
use crate::ui::text::draw_str;
use crate::updater::ReleaseInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

/// A small centered confirm dialog for applying an in-app update. Kept
/// separate from `help_modal.rs`'s style-mate rather than folded into
/// `proc_panel.rs`'s inline `pending_kill` overlay, because this prompt is
/// app-wide: the user can press `u` while any panel is focused, not just
/// the process list.
pub fn render_update_confirm(buf: &mut Buffer, area: Rect, theme: &Theme, release: &ReleaseInfo) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 7u16.min(area.height.saturating_sub(2));
    if width < 20 || height < 5 {
        return;
    }

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_rect = Rect {
        x,
        y,
        width,
        height,
    };

    Clear.render(modal_rect, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.title_focused))
        .title(" Update wtop ")
        .title_style(
            Style::default()
                .fg(theme.title_focused)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal_rect);
    block.render(modal_rect, buf);

    let line1 = format!("A new version is available: v{}", release.version);
    let text_style = Style::default().fg(theme.text_main);
    let dim_style = Style::default().fg(theme.text_dim);

    draw_str(buf, inner.x + 1, inner.y, inner.right(), &line1, text_style);
    draw_str(
        buf,
        inner.x + 1,
        inner.y + 2,
        inner.right(),
        "Download and apply it now?",
        text_style,
    );
    draw_str(
        buf,
        inner.x + 1,
        inner.y + 4,
        inner.right(),
        "Press 'y' to update, 'n' / 'Esc' to cancel",
        dim_style,
    );
}
