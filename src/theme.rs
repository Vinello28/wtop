#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};
use crate::config::ThemeMode;

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub border_normal: Color,
    pub border_focused: Color,
    pub title_focused: Color,
    pub text_main: Color,
    pub text_dim: Color,
    pub text_highlight: Color,
    
    // Module specific accents
    pub cpu_low: Color,
    pub cpu_mid: Color,
    pub cpu_high: Color,
    
    pub mem_primary: Color,
    pub mem_secondary: Color,
    
    pub gpu_primary: Color,
    pub gpu_secondary: Color,
    
    pub net_rx: Color,
    pub net_tx: Color,
    
    pub disk_read: Color,
    pub disk_write: Color,
    
    pub proc_header_bg: Color,
    pub proc_selected_bg: Color,
    pub proc_selected_fg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            border_normal: Color::Rgb(55, 65, 85),
            border_focused: Color::Rgb(0, 220, 255),
            title_focused: Color::Rgb(0, 255, 230),
            text_main: Color::Rgb(226, 232, 240),
            text_dim: Color::Rgb(115, 130, 155),
            text_highlight: Color::Rgb(255, 255, 255),

            cpu_low: Color::Rgb(52, 211, 153),    // Emerald
            cpu_mid: Color::Rgb(251, 191, 36),   // Amber
            cpu_high: Color::Rgb(248, 113, 113), // Coral Red

            mem_primary: Color::Rgb(56, 189, 248),  // Sky blue
            mem_secondary: Color::Rgb(168, 85, 247), // Purple

            gpu_primary: Color::Rgb(217, 70, 239),  // Fuchsia
            gpu_secondary: Color::Rgb(244, 114, 182), // Rose

            net_rx: Color::Rgb(45, 212, 191),   // Teal
            net_tx: Color::Rgb(251, 146, 60),   // Orange

            disk_read: Color::Rgb(96, 165, 250), // Blue
            disk_write: Color::Rgb(244, 63, 94), // Rose red

            proc_header_bg: Color::Rgb(30, 41, 59),
            proc_selected_bg: Color::Rgb(30, 58, 138),
            proc_selected_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            border_normal: Color::Rgb(190, 200, 215),
            border_focused: Color::Rgb(37, 99, 235), // Royal blue
            title_focused: Color::Rgb(29, 78, 216),
            text_main: Color::Rgb(30, 41, 59),
            text_dim: Color::Rgb(100, 116, 139),
            text_highlight: Color::Rgb(15, 23, 42),

            cpu_low: Color::Rgb(22, 163, 74),    // Green
            cpu_mid: Color::Rgb(217, 119, 6),    // Amber
            cpu_high: Color::Rgb(220, 38, 38),   // Crimson

            mem_primary: Color::Rgb(2, 132, 199),   // Deep sky blue
            mem_secondary: Color::Rgb(124, 58, 237), // Indigo

            gpu_primary: Color::Rgb(147, 51, 234),  // Purple
            gpu_secondary: Color::Rgb(190, 24, 93), // Pink

            net_rx: Color::Rgb(13, 148, 136),   // Deep teal
            net_tx: Color::Rgb(194, 65, 12),    // Rust

            disk_read: Color::Rgb(37, 99, 235), // Royal blue
            disk_write: Color::Rgb(185, 28, 28), // Deep red

            proc_header_bg: Color::Rgb(226, 232, 240),
            proc_selected_bg: Color::Rgb(191, 219, 254),
            proc_selected_fg: Color::Rgb(15, 23, 42),
        }
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    pub fn usage_color(&self, pct: f64) -> Color {
        let p = pct.clamp(0.0, 100.0);
        if p < 50.0 {
            let t = p / 50.0;
            interpolate_color(self.cpu_low, self.cpu_mid, t)
        } else {
            let t = (p - 50.0) / 50.0;
            interpolate_color(self.cpu_mid, self.cpu_high, t)
        }
    }

    pub fn block_border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.border_focused).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.border_normal)
        }
    }

    pub fn block_title_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.title_focused).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text_dim)
        }
    }
}

pub fn interpolate_color(c1: Color, c2: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (c1, c2) {
        let r = ((1.0 - t) * r1 as f64 + t * r2 as f64).round() as u8;
        let g = ((1.0 - t) * g1 as f64 + t * g2 as f64).round() as u8;
        let b = ((1.0 - t) * b1 as f64 + t * b2 as f64).round() as u8;
        Color::Rgb(r, g, b)
    } else {
        if t < 0.5 { c1 } else { c2 }
    }
}
