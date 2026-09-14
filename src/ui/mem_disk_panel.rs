use crate::model::{DiskIoData, MemoryData};
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::{format_bytes, format_speed, render_dual_bar, render_mini_bar};
use crate::ui::text::{draw_str, draw_str_with};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_mem_disk_panel(
    buf: &mut Buffer,
    area: Rect,
    mem: &MemoryData,
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
        .title(" MEMORY & DISKS ")
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 3 {
        return;
    }

    // Split vertically: Top for Memory, Bottom for Disks & IO
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let mem_area = sections[0];
    let disk_area = sections[1];

    // --- MEMORY SECTION ---
    if mem_area.height >= 2 {
        let mem_line1 = format!(
            "RAM: {:4.1}%  Used: {} / {}",
            mem.usage_pct,
            format_bytes(mem.used_bytes),
            format_bytes(mem.total_bytes)
        );

        draw_str(
            buf,
            mem_area.x,
            mem_area.y,
            mem_area.right(),
            &mem_line1,
            Style::default()
                .fg(theme.mem_primary)
                .add_modifier(Modifier::BOLD),
        );

        // RAM Usage Bar
        if mem_area.height >= 3 && mem_area.width > 12 {
            let bar_area = Rect {
                x: mem_area.x,
                y: mem_area.y + 1,
                width: (mem_area.width / 2).max(12),
                height: 1,
            };
            render_dual_bar(
                buf,
                bar_area,
                mem.usage_pct,
                theme.mem_primary,
                theme.border_normal,
            );

            // Right side of bar: Braille history if room
            let chart_w = mem_area.width.saturating_sub(bar_area.width + 2);
            if chart_w > 8 {
                let chart_area = Rect {
                    x: mem_area.x + bar_area.width + 2,
                    y: mem_area.y,
                    width: chart_w,
                    height: mem_area.height.min(3),
                };
                let chart =
                    BrailleChart::new(&mem.history, 100.0, theme.mem_primary, theme.mem_secondary);
                chart.render(chart_area, buf);
            }
        }

        // Commit / Swap line
        if mem_area.height >= 3 {
            let swap_line = format!(
                "Commit: {:4.1}%  ({} / {})",
                mem.commit_pct,
                format_bytes(mem.commit_used_bytes),
                format_bytes(mem.commit_total_bytes)
            );
            draw_str(
                buf,
                mem_area.x,
                mem_area.y + 2,
                mem_area.right(),
                &swap_line,
                Style::default().fg(theme.text_dim),
            );
        }
    }

    // --- DISK & IO SECTION ---
    if disk_area.height >= 2 {
        // Disk IO Throughput line
        let io_line = format!(
            "Disk I/O  ▲ {}  ▼ {}",
            format_speed(io.write_bytes_sec),
            format_speed(io.read_bytes_sec)
        );
        draw_str_with(
            buf,
            disk_area.x,
            disk_area.y,
            disk_area.right(),
            &io_line,
            |ch| {
                let color = if ch == '▲' {
                    theme.disk_write
                } else if ch == '▼' {
                    theme.disk_read
                } else {
                    theme.text_main
                };
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            },
        );

        // Partitions list
        for (i, part) in io.partitions.iter().enumerate() {
            let cur_y = disk_area.y + 1 + i as u16;
            if cur_y >= disk_area.bottom() {
                break;
            }

            let part_label = format!("{:<3} ", part.mount);
            let px = draw_str(
                buf,
                disk_area.x,
                cur_y,
                disk_area.right(),
                &part_label,
                Style::default().fg(theme.text_dim),
            );

            let bar_w = (disk_area.width / 4).clamp(6, 14);
            let bar_area = Rect {
                x: px,
                y: cur_y,
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
                cur_y,
                disk_area.right(),
                &details,
                Style::default().fg(theme.text_main),
            );
        }
    }
}
