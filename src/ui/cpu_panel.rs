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
        if num_cores > 0 && right.height > 0 {
            let col_width = 16u16;
            let (num_cols, rows_per_col) =
                core_grid(num_cores, right.width / col_width, right.height as usize);

            for (idx, core) in cpu.cores.iter().enumerate() {
                let col = idx / rows_per_col;
                let row = idx % rows_per_col;

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

/// Lays out `num_cores` per-core rows into a column grid that fills the
/// available width first, instead of only opening a new column once a
/// single column would overflow `available_height` (which produced one
/// tall stack of cores in practice, since panels are usually taller than
/// they have cores). Returns `(num_cols, rows_per_col)`; cores are placed
/// column-major (`col = idx / rows_per_col`, `row = idx % rows_per_col`).
fn core_grid(num_cores: usize, max_cols_by_width: u16, available_height: usize) -> (usize, usize) {
    let num_cols = (max_cols_by_width.max(1) as usize).min(num_cores);
    let rows_per_col = num_cores.div_ceil(num_cols).max(1);
    // If even the widest grid overflows the panel's height, cap rows at
    // the available height; cores beyond num_cols * available_height are
    // dropped by the caller's `col >= num_cols` check, same trade-off the
    // panel always made when it ran out of space.
    (num_cols, rows_per_col.min(available_height.max(1)))
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

    #[test]
    fn core_grid_spreads_across_columns_when_width_allows() {
        // Regression: 16 cores used to collapse into a single 16-row
        // column whenever the panel was taller than 16 rows, even though
        // the panel was wide enough for 3 columns.
        assert_eq!(core_grid(16, 3, 20), (3, 6));
    }

    #[test]
    fn core_grid_single_column_when_too_narrow() {
        assert_eq!(core_grid(8, 1, 20), (1, 8));
    }

    #[test]
    fn core_grid_never_exceeds_available_height() {
        // Way too many cores for the space: rows are capped at the
        // available height, columns stay at the width-derived max.
        assert_eq!(core_grid(100, 3, 10), (3, 10));
    }

    #[test]
    fn core_grid_does_not_open_empty_columns() {
        // Fewer cores than columns the width would allow: don't spread
        // 4 cores across 6 columns.
        assert_eq!(core_grid(4, 6, 20), (4, 1));
    }

    #[test]
    fn core_grid_single_core() {
        assert_eq!(core_grid(1, 4, 20), (1, 1));
    }
}
