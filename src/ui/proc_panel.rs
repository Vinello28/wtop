use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, BorderType, Widget};
use crate::config::ProcessSortBy;
use crate::model::ProcessItem;
use crate::theme::Theme;
use crate::ui::gauge::format_bytes;

pub fn render_proc_panel(
    buf: &mut Buffer,
    area: Rect,
    processes: &[ProcessItem],
    selected_idx: usize,
    scroll_offset: usize,
    sort_by: ProcessSortBy,
    desc: bool,
    filter: &str,
    is_filtering: bool,
    pending_kill: Option<u32>,
    theme: &Theme,
    focused: bool,
) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let sort_str = match sort_by {
        ProcessSortBy::Cpu => if desc { "CPU ▼" } else { "CPU ▲" },
        ProcessSortBy::Memory => if desc { "MEM ▼" } else { "MEM ▲" },
        ProcessSortBy::Pid => if desc { "PID ▼" } else { "PID ▲" },
        ProcessSortBy::Name => if desc { "NAME ▼" } else { "NAME ▲" },
    };

    let title = if is_filtering {
        format!(" PROCESSES [Find: {}_] ({}) ", filter, processes.len())
    } else if !filter.is_empty() {
        format!(" PROCESSES [Filter: '{}'] ({} matches) [Sort: {}] ", filter, processes.len(), sort_str)
    } else {
        format!(" PROCESSES ({}) [Sort: {}] ", processes.len(), sort_str)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_border_style(focused))
        .title(title)
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 20 || inner.height < 2 {
        return;
    }

    // Header row
    let header_y = inner.y;
    let header_bg_style = Style::default().bg(theme.proc_header_bg).add_modifier(Modifier::BOLD);

    // Fill header background
    for x in inner.x..inner.right() {
        buf[(x, header_y)].set_char(' ').set_style(header_bg_style);
    }

    let col_pid = format!("{:<7}", "PID");
    let col_cpu = format!("{:>7}", if matches!(sort_by, ProcessSortBy::Cpu) { "CPU% *" } else { "CPU%" });
    let col_mem = format!("{:>10}", if matches!(sort_by, ProcessSortBy::Memory) { "MEM *" } else { "MEM" });
    let col_thr = format!("{:>5}", "THR");
    let fixed_w = 7 + 7 + 10 + 5 + 4; // 33
    let name_w = inner.width.saturating_sub(fixed_w as u16) as usize;

    let col_name = format!("{:<width$}", "NAME", width = name_w);

    let mut cur_x = inner.x;
    for (col_text, style) in [
        (&col_pid, Style::default().fg(theme.title_focused).bg(theme.proc_header_bg).add_modifier(Modifier::BOLD)),
        (&col_name, Style::default().fg(theme.text_main).bg(theme.proc_header_bg).add_modifier(Modifier::BOLD)),
        (&col_cpu, Style::default().fg(theme.cpu_mid).bg(theme.proc_header_bg).add_modifier(Modifier::BOLD)),
        (&col_mem, Style::default().fg(theme.mem_primary).bg(theme.proc_header_bg).add_modifier(Modifier::BOLD)),
        (&col_thr, Style::default().fg(theme.text_dim).bg(theme.proc_header_bg).add_modifier(Modifier::BOLD)),
    ] {
        for ch in col_text.chars() {
            if cur_x < inner.right() {
                buf[(cur_x, header_y)].set_char(ch).set_style(style);
                cur_x += 1;
            }
        }
    }

    // Rows
    let rows_visible = inner.height.saturating_sub(1) as usize;
    let end_idx = (scroll_offset + rows_visible).min(processes.len());

    for (row_i, proc_idx) in (scroll_offset..end_idx).enumerate() {
        let p = &processes[proc_idx];
        let y = inner.y + 1 + row_i as u16;
        let is_selected = proc_idx == selected_idx;

        let row_style = if is_selected {
            Style::default()
                .bg(theme.proc_selected_bg)
                .fg(theme.proc_selected_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_main)
        };

        // Fill background if selected
        if is_selected {
            for x in inner.x..inner.right() {
                buf[(x, y)].set_char(' ').set_style(row_style);
            }
        }

        // Draw columns
        let pid_str = format!("{:<7}", p.pid);
        let name_str = format!("{:<width$}", truncate_str(&p.name, name_w), width = name_w);
        let cpu_str = format!("{:>6.1}%", p.cpu_pct);
        let mem_str = format!("{:>10}", format_bytes(p.mem_bytes));
        let thr_str = format!("{:>5}", p.threads);

        let mut rx = inner.x;

        for (text, col_style) in [
            (&pid_str, if is_selected { row_style } else { Style::default().fg(theme.text_dim) }),
            (&name_str, if is_selected { row_style } else { Style::default().fg(theme.text_main) }),
            (&cpu_str, if is_selected { row_style } else { Style::default().fg(theme.usage_color(p.cpu_pct as f64)) }),
            (&mem_str, if is_selected { row_style } else { Style::default().fg(theme.mem_primary) }),
            (&thr_str, if is_selected { row_style } else { Style::default().fg(theme.text_dim) }),
        ] {
            for ch in text.chars() {
                if rx < inner.right() {
                    buf[(rx, y)].set_char(ch).set_style(col_style);
                    rx += 1;
                }
            }
        }
    }

    // Kill confirmation prompt overlay if active
    if let Some(target_pid) = pending_kill {
        let kill_text = format!(" Terminate PID {}? Press 'y' to Kill, 'n'/'Esc' to Cancel ", target_pid);
        let k_len = kill_text.len() as u16;
        let kx = inner.x + (inner.width.saturating_sub(k_len)) / 2;
        let ky = inner.y + inner.height / 2;

        let kill_style = Style::default()
            .bg(theme.cpu_high)
            .fg(ratatui::style::Color::White)
            .add_modifier(Modifier::BOLD);

        let mut cx = kx;
        for ch in kill_text.chars() {
            if cx < inner.right() {
                buf[(cx, ky)].set_char(ch).set_style(kill_style);
                cx += 1;
            }
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len && max_len > 3 {
        let mut res = String::new();
        for ch in s.chars().take(max_len - 3) {
            res.push(ch);
        }
        res.push_str("...");
        res
    } else {
        s.to_string()
    }
}
