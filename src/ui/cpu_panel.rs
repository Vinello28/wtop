use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, BorderType, Widget};
use crate::model::CpuData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::render_mini_bar;

pub fn render_cpu_panel(
    buf: &mut Buffer,
    area: Rect,
    cpu: &CpuData,
    theme: &Theme,
    focused: bool,
) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let title = if cpu.model_name.is_empty() {
        " CPU ".to_string()
    } else {
        format!(" CPU: {} ", cpu.model_name)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_border_style(focused))
        .title(title)
        .title_style(theme.block_title_style(focused));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 8 || inner.height < 2 {
        return;
    }

    // Split inner into Left (Braille Chart + Global usage) and Right (Per-Core bars)
    let chunks = if inner.width > 60 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Percentage(0)])
            .split(inner)
    };

    // Left: Global CPU Braille Graph
    let left = chunks[0];
    if left.height >= 2 {
        // Line 0: Global summary text
        let usage_color = theme.usage_color(cpu.global_pct);
        let summary_text = format!("Total: {:5.1}%", cpu.global_pct);
        let mut x = left.x;
        for ch in summary_text.chars() {
            if x < left.right() {
                buf[(x, left.y)]
                    .set_char(ch)
                    .set_style(Style::default().fg(usage_color).add_modifier(Modifier::BOLD));
                x += 1;
            }
        }

        // Braille Chart below summary
        let chart_area = Rect {
            x: left.x,
            y: left.y + 1,
            width: left.width,
            height: left.height.saturating_sub(1),
        };

        if chart_area.height > 0 {
            let chart = BrailleChart::new(&cpu.history, 100.0, theme.cpu_low, theme.cpu_high);
            chart.render(chart_area, buf);
        }
    }

    // Right: Per-Core mini-bars
    if chunks.len() > 1 && chunks[1].width > 12 {
        let right = chunks[1];
        let num_cores = cpu.cores.len();
        if num_cores > 0 {
            let col_width = 16u16;
            let num_cols = (right.width / col_width).max(1) as usize;
            let rows_available = right.height as usize;

            for (idx, core) in cpu.cores.iter().enumerate() {
                let col = idx / rows_available;
                let row = idx % rows_available;

                if col >= num_cols {
                    break;
                }

                let x = right.x + (col as u16 * col_width);
                let y = right.y + row as u16;

                if y < right.bottom() && x + col_width <= right.right() + 1 {
                    let label = format!("{:02}:", core.id);
                    let mut lx = x;
                    for ch in label.chars() {
                        buf[(lx, y)].set_char(ch).set_style(Style::default().fg(theme.text_dim));
                        lx += 1;
                    }

                    let bar_area = Rect {
                        x: x + 4,
                        y,
                        width: 6,
                        height: 1,
                    };
                    render_mini_bar(buf, bar_area, core.usage_pct, theme, '·');

                    let pct_str = format!("{:3.0}%", core.usage_pct);
                    let mut px = x + 11;
                    let color = theme.usage_color(core.usage_pct);
                    for ch in pct_str.chars() {
                        if px < right.right() {
                            buf[(px, y)].set_char(ch).set_style(Style::default().fg(color));
                            px += 1;
                        }
                    }
                }
            }
        }
    }
}
