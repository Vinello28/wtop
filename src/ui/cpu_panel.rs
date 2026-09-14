use crate::model::CpuData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::render_mini_bar;
use crate::ui::text::{draw_str, truncate};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// Chars taken by the fixed `" CPU:  "` title frame (leading space, "CPU:",
/// two spaces, trailing space) around the variable-length model name.
const TITLE_FRAME_LEN: usize = 7;

pub fn render_cpu_panel(buf: &mut Buffer, area: Rect, cpu: &CpuData, theme: &Theme, focused: bool) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let title = cpu_title(&cpu.model_name, area.width);

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
        draw_str(
            buf,
            left.x,
            left.y,
            left.right(),
            &summary_text,
            Style::default()
                .fg(usage_color)
                .add_modifier(Modifier::BOLD),
        );

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
                    draw_str(
                        buf,
                        x,
                        y,
                        right.right(),
                        &label,
                        Style::default().fg(theme.text_dim),
                    );

                    let bar_area = Rect {
                        x: x + 4,
                        y,
                        width: 6,
                        height: 1,
                    };
                    render_mini_bar(buf, bar_area, core.usage_pct, theme, '·');

                    let pct_str = format!("{:3.0}%", core.usage_pct);
                    let color = theme.usage_color(core.usage_pct);
                    draw_str(
                        buf,
                        x + 11,
                        y,
                        right.right(),
                        &pct_str,
                        Style::default().fg(color),
                    );
                }
            }
        }
    }
}

/// Builds the CPU panel's block title, truncating `model_name` to fit the
/// available border width with an ellipsis rather than letting ratatui
/// silently clip it. Falls back to a bare " CPU " title when the panel is
/// too narrow to show a meaningful fragment of the name.
fn cpu_title(model_name: &str, area_width: u16) -> String {
    if model_name.is_empty() {
        return " CPU ".to_string();
    }
    let avail = area_width.saturating_sub(2) as usize; // inside the left/right border
    let name_budget = avail.saturating_sub(TITLE_FRAME_LEN);
    if name_budget < 4 {
        return " CPU ".to_string();
    }
    format!(" CPU: {} ", truncate(model_name, name_budget))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_title_fits_full_name() {
        assert_eq!(cpu_title("Ryzen 9", 30), " CPU: Ryzen 9 ");
    }

    #[test]
    fn cpu_title_truncates_long_name() {
        let title = cpu_title("AMD Ryzen 9 7950X 16-Core Processor", 25);
        assert!(title.starts_with(" CPU: "));
        assert!(title.contains('…'));
        assert!(title.chars().count() as u16 <= 25);
    }

    #[test]
    fn cpu_title_falls_back_when_too_narrow() {
        assert_eq!(cpu_title("Ryzen 9", 10), " CPU ");
    }

    #[test]
    fn cpu_title_empty_name() {
        assert_eq!(cpu_title("", 30), " CPU ");
    }
}
