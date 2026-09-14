use crate::theme::Theme;
use crate::ui::text::draw_str;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

pub fn render_help_modal(buf: &mut Buffer, area: Rect, theme: &Theme) {
    let width = 64u16.min(area.width.saturating_sub(4));
    let height = 22u16.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_rect = Rect {
        x,
        y,
        width,
        height,
    };

    // Clear background
    Clear.render(modal_rect, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.title_focused))
        .title(" wtop - Keyboard Shortcuts & Help ")
        .title_style(
            Style::default()
                .fg(theme.title_focused)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal_rect);
    block.render(modal_rect, buf);

    let shortcuts = [
        ("Tab / Shift-Tab", "Switch active panel"),
        ("Up / Down (j / k)", "Navigate processes list"),
        ("PgUp / PgDn", "Scroll processes page"),
        ("Home / End", "Jump to top / bottom of list"),
        ("c / m / p / n", "Sort processes by CPU / MEM / PID / Name"),
        ("d", "Reverse sorting order (Asc / Desc)"),
        ("/", "Filter / search processes in real time"),
        ("x / Delete", "Terminate selected process (Kill)"),
        ("t", "Toggle Theme (Dark ↔ Light)"),
        ("+ / -", "Increase / decrease sampling interval"),
        ("u", "Check for / apply an available update"),
        ("g", "Open the developer's GitHub profile in your browser"),
        ("? / h", "Toggle this Help window"),
        ("q / Esc", "Quit wtop / Cancel current action"),
    ];

    let mut row_y = inner.y + 1;
    for (key, desc) in shortcuts {
        if row_y >= inner.bottom() {
            break;
        }

        let key_style = Style::default()
            .fg(theme.title_focused)
            .add_modifier(Modifier::BOLD);
        draw_str(buf, inner.x + 2, row_y, inner.right(), key, key_style);

        let desc_style = Style::default().fg(theme.text_main);
        draw_str(buf, inner.x + 24, row_y, inner.right(), desc, desc_style);

        row_y += 1;
    }

    // Bottom tip
    if inner.bottom() > row_y + 1 {
        let tip = "Press 'Esc' or '?' to close this help window";
        let tip_x = inner.x + (inner.width.saturating_sub(tip.chars().count() as u16)) / 2;
        let tip_y = inner.bottom() - 1;
        let tip_style = Style::default().fg(theme.text_dim);
        draw_str(buf, tip_x, tip_y, inner.right(), tip, tip_style);
    }
}
