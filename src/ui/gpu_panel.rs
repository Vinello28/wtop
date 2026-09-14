use crate::model::GpuData;
use crate::theme::Theme;
use crate::ui::braille::BrailleChart;
use crate::ui::gauge::format_bytes;
use crate::ui::text::{draw_str, truncate};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub fn render_gpu_panel(buf: &mut Buffer, area: Rect, gpu: &GpuData, theme: &Theme, focused: bool) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.block_border_style(focused))
        .title(" GPU ")
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

    // Reserve space for the fixed percentage/VRAM part first, then give
    // whatever remains to the (potentially long) adapter name -- never
    // truncate the composed line blindly from the end.
    let fixed_part = format!("GPU: {:5.1}%  {}  ", gpu.utilization_pct, vram_str);
    let name_budget = (inner.width as usize).saturating_sub(fixed_part.chars().count());
    let header = format!("{}{}", fixed_part, truncate(&gpu.name, name_budget));

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
