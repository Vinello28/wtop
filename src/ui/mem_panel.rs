use crate::model::MemoryData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::{format_bytes, render_dual_bar};
use crate::ui::text::draw_str;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_mem_panel(
    buf: &mut Buffer,
    area: Rect,
    mem: &MemoryData,
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
        .title(" MEMORY ")
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 2 {
        return;
    }

    let line1 = format!(
        "RAM: {:4.1}%  Used: {} / {}",
        mem.usage_pct,
        format_bytes(mem.used_bytes),
        format_bytes(mem.total_bytes)
    );
    draw_str(
        buf,
        inner.x,
        inner.y,
        inner.right(),
        &line1,
        Style::default()
            .fg(theme.mem_primary)
            .add_modifier(Modifier::BOLD),
    );

    if inner.height >= 2 {
        let bar_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 1,
        };
        render_dual_bar(
            buf,
            bar_area,
            mem.usage_pct,
            theme.mem_primary,
            theme.border_normal,
        );
    }

    if inner.height >= 3 {
        let swap_line = format!(
            "Commit: {:4.1}%  ({} / {})",
            mem.commit_pct,
            format_bytes(mem.commit_used_bytes),
            format_bytes(mem.commit_total_bytes)
        );
        draw_str(
            buf,
            inner.x,
            inner.y + 2,
            inner.right(),
            &swap_line,
            Style::default().fg(theme.text_dim),
        );
    }

    // Freed from having to share space with the disk section, memory now
    // gets a full-width history graph across whatever height remains.
    if inner.height > 3 {
        let chart_area = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: inner.height - 3,
        };
        let chart = BrailleChart::new(&mem.history, 100.0, theme.mem_primary, theme.mem_secondary);
        chart.render(chart_area, buf);
    }
}
