use crate::model::{CpuCore, CpuData};
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

    const COL_WIDTH: u16 = 16;
    let num_cores = cpu.cores.len();

    // Try the wide layout first: chart+total on the left, per-core grid on
    // the right. Only worth it once the 45% right slice actually fits two
    // or more core columns -- otherwise it wastes width on a single column
    // (or, below the old 60-col threshold, hid the grid entirely). In both
    // of those cases the stacked layout below makes better use of the
    // space, since it gives the grid the panel's full width instead of a
    // slice of it.
    let side_by_side = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    if num_cores > 0 && side_by_side[1].width >= COL_WIDTH * 2 {
        render_summary(buf, side_by_side[0], cpu, theme);
        render_core_grid(buf, side_by_side[1], &cpu.cores, theme, COL_WIDTH);
    } else if num_cores > 0 && inner.width >= COL_WIDTH {
        // Stack the grid (full width, so it can still spread across
        // multiple columns) below the chart instead of hiding cores.
        let (top_height, core_height) =
            stacked_split(num_cores, inner.width, inner.height, COL_WIDTH);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_height),
                Constraint::Length(core_height),
            ])
            .split(inner);

        render_summary(buf, rows[0], cpu, theme);
        if core_height > 0 {
            render_core_grid(buf, rows[1], &cpu.cores, theme, COL_WIDTH);
        }
    } else {
        // Not enough width for even one core column: total + chart only.
        render_summary(buf, inner, cpu, theme);
    }
}

/// Renders the "Total: NN.N%" line plus the braille history chart into
/// `area`. Shared by the side-by-side and stacked CPU panel layouts.
fn render_summary(buf: &mut Buffer, area: Rect, cpu: &CpuData, theme: &Theme) {
    if area.height == 0 {
        return;
    }

    let usage_color = theme.usage_color(cpu.global_pct);
    let summary_text = format!("Total: {:5.1}%", cpu.global_pct);
    draw_str(
        buf,
        area.x,
        area.y,
        area.right(),
        &summary_text,
        Style::default()
            .fg(usage_color)
            .add_modifier(Modifier::BOLD),
    );

    let chart_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    if chart_area.height > 0 {
        let chart = BrailleChart::new(&cpu.history, 100.0, theme.cpu_low, theme.cpu_high);
        chart.render(chart_area, buf);
    }
}

/// Renders `cores` as a multi-column grid of "`id`: [bar] pct%" cells
/// filling `area`, using `core_grid` to fit as many columns as `area`'s
/// width allows before stacking into further rows.
fn render_core_grid(
    buf: &mut Buffer,
    area: Rect,
    cores: &[CpuCore],
    theme: &Theme,
    col_width: u16,
) {
    if area.height == 0 {
        return;
    }
    let (num_cols, rows_per_col) =
        core_grid(cores.len(), area.width / col_width, area.height as usize);

    for (idx, core) in cores.iter().enumerate() {
        let col = idx / rows_per_col;
        let row = idx % rows_per_col;

        if col >= num_cols {
            break;
        }

        let x = area.x + (col as u16 * col_width);
        let y = area.y + row as u16;

        if y < area.bottom() && x + col_width <= area.right() + 1 {
            let label = format!("{:02}:", core.id);
            draw_str(
                buf,
                x,
                y,
                area.right(),
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
                area.right(),
                &pct_str,
                Style::default().fg(color),
            );
        }
    }
}

/// Splits `height` between the total/chart summary (top) and the per-core
/// grid (bottom) for the stacked layout. The grid gets exactly the rows
/// `core_grid` says it needs for `width`, capped at what's left after
/// reserving one row for the "Total:" line; the chart gets whatever
/// remains, down to nothing when the grid needs it all.
fn stacked_split(num_cores: usize, width: u16, height: u16, col_width: u16) -> (u16, u16) {
    if height == 0 || num_cores == 0 {
        return (height, 0);
    }
    let available_for_grid = height.saturating_sub(1) as usize;
    let (_, rows_per_col) = core_grid(num_cores, width / col_width, available_for_grid);
    let core_height = rows_per_col.min(available_for_grid) as u16;
    (height - core_height, core_height)
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

    #[test]
    fn stacked_split_uses_full_rows_when_height_allows() {
        // 8 cores at width 40 -> 2 columns (40/16), 4 rows needed; height
        // 10 leaves 9 rows after the total line, more than enough.
        assert_eq!(stacked_split(8, 40, 10, 16), (6, 4));
    }

    #[test]
    fn stacked_split_caps_grid_when_height_is_tight() {
        // Same 8 cores / 2 columns need 4 rows, but only 2 are left after
        // the total line: the grid takes all of it, the chart gets none.
        assert_eq!(stacked_split(8, 40, 3, 16), (1, 2));
    }

    #[test]
    fn stacked_split_no_cores_keeps_full_height_for_summary() {
        assert_eq!(stacked_split(0, 40, 10, 16), (10, 0));
    }

    #[test]
    fn stacked_split_zero_height_is_a_noop() {
        assert_eq!(stacked_split(8, 40, 0, 16), (0, 0));
    }

    fn render_to_string(buf: &Buffer, area: Rect) -> String {
        (area.y..area.bottom())
            .map(|y| {
                (area.x..area.right())
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn cpu_panel_keeps_cores_visible_when_narrower_than_old_threshold() {
        // Regression: below the old 60-col side-by-side threshold, the
        // per-core grid used to be dropped entirely (right chunk got
        // Percentage(0)) and only the total was shown.
        let theme = Theme::dark();
        let cores = (0..8)
            .map(|id| CpuCore {
                id,
                usage_pct: 10.0 * id as f64,
            })
            .collect();
        let cpu = CpuData {
            model_name: "Test CPU".to_string(),
            global_pct: 42.0,
            cores,
            history: vec![10.0, 20.0, 30.0],
        };

        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        render_cpu_panel(&mut buf, area, &cpu, &theme, false);

        let rendered = render_to_string(&buf, area);
        assert!(rendered.contains("00:"), "core grid missing: {rendered:?}");
        assert!(rendered.contains("04:"), "core grid missing: {rendered:?}");

        // Multi-column: core 0 and core 4 land in the same row
        // (column-major layout, 2 columns x 4 rows), never in a single
        // stacked column.
        let row_with_core0 = rendered
            .lines()
            .find(|line| line.contains("00:"))
            .expect("row with core 00 not found");
        assert!(
            row_with_core0.contains("04:"),
            "cores are not spread across multiple columns: {row_with_core0:?}"
        );
    }
}
