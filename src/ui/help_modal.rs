use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, BorderType, Clear, Widget};
use crate::theme::Theme;

pub fn render_help_modal(buf: &mut Buffer, area: Rect, theme: &Theme) {
    let width = 64u16.min(area.width.saturating_sub(4));
    let height = 22u16.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_rect = Rect { x, y, width, height };

    // Clear background
    Clear.render(modal_rect, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.title_focused))
        .title(" wtop - Keyboard Shortcuts & Help ")
        .title_style(Style::default().fg(theme.title_focused).add_modifier(Modifier::BOLD));

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
        ("? / h", "Toggle this Help window"),
        ("q / Esc", "Quit wtop / Cancel current action"),
    ];

    let mut row_y = inner.y + 1;
    for (key, desc) in shortcuts {
        if row_y >= inner.bottom() {
            break;
        }

        let mut kx = inner.x + 2;
        let key_style = Style::default().fg(theme.title_focused).add_modifier(Modifier::BOLD);
        for ch in key.chars() {
            if kx < inner.right() {
                buf[(kx, row_y)].set_char(ch).set_style(key_style);
                kx += 1;
            }
        }

        let mut dx = inner.x + 24;
        let desc_style = Style::default().fg(theme.text_main);
        for ch in desc.chars() {
            if dx < inner.right() {
                buf[(dx, row_y)].set_char(ch).set_style(desc_style);
                dx += 1;
            }
        }

        row_y += 1;
    }

    // Bottom tip
    if inner.bottom() > row_y + 1 {
        let tip = "Press 'Esc' or '?' to close this help window";
        let tip_x = inner.x + (inner.width.saturating_sub(tip.len() as u16)) / 2;
        let tip_y = inner.bottom() - 1;
        let tip_style = Style::default().fg(theme.text_dim);
        for (i, ch) in tip.chars().enumerate() {
            let cx = tip_x + i as u16;
            if cx < inner.right() {
                buf[(cx, tip_y)].set_char(ch).set_style(tip_style);
            }
        }
    }
}
