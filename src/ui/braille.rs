use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;
use crate::theme::interpolate_color;

pub struct BrailleChart<'a> {
    data: &'a [f64],
    max_val: f64,
    color_low: Color,
    color_high: Color,
    filled: bool,
}

impl<'a> BrailleChart<'a> {
    pub fn new(data: &'a [f64], max_val: f64, color_low: Color, color_high: Color) -> Self {
        Self {
            data,
            max_val: if max_val <= 0.0 { 100.0 } else { max_val },
            color_low,
            color_high,
            filled: true,
        }
    }

    #[allow(dead_code)]
    pub fn line_only(mut self) -> Self {
        self.filled = false;
        self
    }
}

impl<'a> Widget for BrailleChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 1 || area.height < 1 {
            return;
        }

        let width_chars = area.width as usize;
        let height_chars = area.height as usize;
        let num_sub_cols = width_chars * 2;
        let total_sub_rows = height_chars * 4;

        // Extract latest points that fit into num_sub_cols
        let mut samples = vec![0.0; num_sub_cols];
        let data_len = self.data.len();
        if data_len > 0 {
            let start = if data_len >= num_sub_cols {
                data_len - num_sub_cols
            } else {
                0
            };
            let slice = &self.data[start..];
            let offset = num_sub_cols.saturating_sub(slice.len());
            for (i, &val) in slice.iter().enumerate() {
                samples[offset + i] = val.clamp(0.0, self.max_val);
            }
        }

        // Map samples to sub-row heights (0..total_sub_rows)
        let mut heights = vec![0usize; num_sub_cols];
        for (i, &val) in samples.iter().enumerate() {
            let ratio = (val / self.max_val).clamp(0.0, 1.0);
            heights[i] = (ratio * total_sub_rows as f64).round() as usize;
        }

        // Left column dot masks by dot index (0..3 from bottom to top):
        // dot 7 = 0x40, dot 3 = 0x04, dot 2 = 0x02, dot 1 = 0x01
        let left_dots = [0x40u8, 0x04, 0x02, 0x01];
        // Right column dot masks (dot 8 = 0x80, dot 6 = 0x20, dot 5 = 0x10, dot 4 = 0x08)
        let right_dots = [0x80u8, 0x20, 0x10, 0x08];

        for char_y in 0..height_chars {
            // char_y = 0 is top row, char_y = height_chars - 1 is bottom row
            let row_from_bottom = height_chars - 1 - char_y;
            let sub_row_base = row_from_bottom * 4;

            let row_ratio = if height_chars > 1 {
                row_from_bottom as f64 / (height_chars - 1) as f64
            } else {
                1.0
            };
            let cell_color = interpolate_color(self.color_low, self.color_high, row_ratio);

            for char_x in 0..width_chars {
                let left_sub_col = char_x * 2;
                let right_sub_col = left_sub_col + 1;

                let left_h = heights[left_sub_col];
                let right_h = heights[right_sub_col];

                let mut braille_byte = 0u8;

                // Render 4 vertical dots for left and right
                for dot_idx in 0..4 {
                    let dot_sub_row = sub_row_base + dot_idx + 1;

                    let left_active = if self.filled {
                        dot_sub_row <= left_h
                    } else {
                        // Outline/line only
                        dot_sub_row == left_h || (dot_idx == 0 && dot_sub_row <= left_h && left_h > 0)
                    };

                    let right_active = if self.filled {
                        dot_sub_row <= right_h
                    } else {
                        dot_sub_row == right_h || (dot_idx == 0 && dot_sub_row <= right_h && right_h > 0)
                    };

                    if left_active {
                        braille_byte |= left_dots[dot_idx];
                    }
                    if right_active {
                        braille_byte |= right_dots[dot_idx];
                    }
                }

                let ch = if braille_byte == 0 {
                    ' '
                } else {
                    char::from_u32(0x2800 + braille_byte as u32).unwrap_or(' ')
                };

                let screen_x = area.x + char_x as u16;
                let screen_y = area.y + char_y as u16;
                buf[(screen_x, screen_y)]
                    .set_char(ch)
                    .set_style(Style::default().fg(cell_color));
            }
        }
    }
}
