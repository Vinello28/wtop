use crate::model::DiskIoData;
use crate::theme::Theme;
use crate::ui::braille::{StackedSeries, render_stacked_charts};
use crate::ui::gauge::{format_bytes, format_speed, render_mini_bar};
use crate::ui::text::{draw_str, draw_str_with};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_disk_panel(
    buf: &mut Buffer,
    area: Rect,
    io: &DiskIoData,
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
        .title(" DISKS & I/O ")
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 2 {
        return;
    }

    let io_line = format!(
        "Disk I/O  ▲ {}  ▼ {}",
        format_speed(io.write_bytes_sec),
        format_speed(io.read_bytes_sec)
    );
    draw_str_with(buf, inner.x, inner.y, inner.right(), &io_line, |ch| {
        let color = if ch == '▲' {
            theme.disk_write
        } else if ch == '▼' {
            theme.disk_read
        } else {
            theme.text_main
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    });

    let partitions_h = io.partitions.len() as u16;
    let body_h = inner.height.saturating_sub(1);

    // History chart gets whatever's left after reserving one row per
    // partition (the partition list always wins the space -- it's the part
    // that was already here), capped at 3 rows so it never crowds out a
    // short partition list either.
    let chart_h = body_h.saturating_sub(partitions_h).min(3);

    let mut cur_y = inner.y + 1;
    if chart_h >= 2 {
        let chart_area = Rect {
            x: inner.x,
            y: cur_y,
            width: inner.width,
            height: chart_h,
        };
        render_stacked_charts(
            buf,
            chart_area,
            StackedSeries {
                data: &io.read_history,
                low: theme.border_normal,
                high: theme.disk_read,
            },
            StackedSeries {
                data: &io.write_history,
                low: theme.border_normal,
                high: theme.disk_write,
            },
        );
        cur_y += chart_h;
    }

    for part in io.partitions.iter() {
        let y = cur_y;
        if y >= inner.bottom() {
            break;
        }
        cur_y += 1;

        let part_label = format!("{:<3} ", part.mount);
        let px = draw_str(
            buf,
            inner.x,
            y,
            inner.right(),
            &part_label,
            Style::default().fg(theme.text_dim),
        );

        let bar_w = (inner.width / 4).clamp(6, 14);
        let bar_area = Rect {
            x: px,
            y,
            width: bar_w,
            height: 1,
        };
        render_mini_bar(buf, bar_area, part.usage_pct, theme, '·');

        let details = format!(
            " {:4.1}%  {} free / {}",
            part.usage_pct,
            format_bytes(part.free_bytes),
            format_bytes(part.total_bytes)
        );
        draw_str(
            buf,
            px + bar_w,
            y,
            inner.right(),
            &details,
            Style::default().fg(theme.text_main),
        );
    }
}
