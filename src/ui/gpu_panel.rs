use crate::model::GpuData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::format_bytes;
use crate::ui::text::{draw_str, truncate};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// Chars taken by the fixed `" GPU:  "` title frame (mirrors `cpu_panel`'s
/// `TITLE_FRAME_LEN` -- "CPU" and "GPU" are the same length, so the frame is
/// identical).
const TITLE_FRAME_LEN: usize = 7;

pub fn render_gpu_panel(buf: &mut Buffer, area: Rect, gpu: &GpuData, theme: &Theme, focused: bool) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let title = gpu_title(&gpu.name, area.width);

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

    let vram_str = if gpu.dedicated_vram_used > 0 {
        format!("VRAM: {}", format_bytes(gpu.dedicated_vram_used))
    } else {
        String::new()
    };

    // The adapter name now lives in the title (like CPU/NETWORK), so the
    // body only needs the percentage and, when known, VRAM usage.
    let header = if vram_str.is_empty() {
        format!("GPU: {:5.1}%", gpu.utilization_pct)
    } else {
        format!("GPU: {:5.1}%  {}", gpu.utilization_pct, vram_str)
    };

    draw_str(
        buf,
        inner.x,
        inner.y,
        inner.right(),
        &header,
        Style::default()
            .fg(theme.gpu_primary)
            .add_modifier(Modifier::BOLD),
    );

    // Freed from sharing the panel with the network section, GPU now gets a
    // full-width, full-height history graph.
    let chart_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };

    if chart_area.height > 0 {
        let chart = BrailleChart::new(&gpu.history, 100.0, theme.gpu_primary, theme.gpu_secondary);
        chart.render(chart_area, buf);
    }
}

/// Builds the GPU panel's block title from the adapter name, truncating with
/// an ellipsis rather than letting ratatui silently clip it. Falls back to a
/// bare " GPU " title when there's no name or no room. Mirrors `cpu_title`
/// (`cpu_panel.rs`) and `net_title` (`net_panel.rs`) exactly, so all three
/// device panels present their hardware name the same way.
fn gpu_title(name: &str, area_width: u16) -> String {
    if name.is_empty() {
        return " GPU ".to_string();
    }
    let avail = area_width.saturating_sub(2) as usize; // inside the left/right border
    let name_budget = avail.saturating_sub(TITLE_FRAME_LEN);
    if name_budget < 4 {
        return " GPU ".to_string();
    }
    format!(" GPU: {} ", truncate(name, name_budget))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_title_fits_full_name() {
        assert_eq!(gpu_title("RTX 4090", 30), " GPU: RTX 4090 ");
    }

    #[test]
    fn gpu_title_truncates_long_name() {
        let title = gpu_title("NVIDIA GeForce RTX 4090 Laptop GPU", 25);
        assert!(title.starts_with(" GPU: "));
        assert!(title.contains('…'));
        assert!(title.chars().count() as u16 <= 25);
    }

    #[test]
    fn gpu_title_falls_back_when_too_narrow() {
        assert_eq!(gpu_title("RTX 4090", 10), " GPU ");
    }

    #[test]
    fn gpu_title_empty_name() {
        assert_eq!(gpu_title("", 30), " GPU ");
    }

    #[test]
    fn gpu_panel_keeps_name_out_of_body() {
        // Regression: the adapter name used to be appended to the body's
        // percentage/VRAM line instead of living in the title like every
        // other device panel (CPU model, NETWORK adapter).
        let theme = Theme::dark();
        let gpu = GpuData {
            name: "NVIDIA GeForce RTX 4090".to_string(),
            utilization_pct: 12.3,
            dedicated_vram_used: 512 * 1024 * 1024,
            ..Default::default()
        };

        let area = Rect::new(0, 0, 50, 10);
        let mut buf = Buffer::empty(area);
        render_gpu_panel(&mut buf, area, &gpu, &theme, false);

        let rendered: String = (area.y..area.bottom())
            .map(|y| {
                (area.x..area.right())
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        let body: String = rendered.lines().skip(1).collect::<Vec<_>>().join("\n");
        assert!(
            !body.contains("RTX 4090"),
            "adapter name leaked into the panel body: {body:?}"
        );
        assert!(
            rendered.lines().next().unwrap().contains("RTX 4090"),
            "adapter name missing from the title row: {rendered:?}"
        );
    }
}
