use ratatui::buffer::Buffer;
use ratatui::style::Style;
use std::borrow::Cow;

/// Truncate `s` to at most `max_chars` characters, appending an ellipsis when
/// truncated. UTF-8 safe (counts chars, never splits a codepoint). Returns a
/// borrowed `Cow` when `s` already fits, so callers pay no allocation in the
/// common case of a name that fits its budget.
pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str> {
    if s.chars().count() <= max_chars {
        return Cow::Borrowed(s);
    }
    if max_chars == 0 {
        return Cow::Borrowed("");
    }
    if max_chars == 1 {
        return Cow::Owned("…".to_string());
    }
    let mut truncated: String = s.chars().take(max_chars - 1).collect();
    truncated.push('…');
    Cow::Owned(truncated)
}

/// Draw `s` left-to-right starting at `(x, y)`, clipping any character that
/// would land at or past `max_x`. Returns the x position just past the last
/// drawn cell, so callers can chain multiple styled segments on one row.
pub fn draw_str(buf: &mut Buffer, x: u16, y: u16, max_x: u16, s: &str, style: Style) -> u16 {
    draw_str_with(buf, x, y, max_x, s, |_| style)
}

/// Same as [`draw_str`], but the style is computed per character. Useful when
/// a line mixes colors by glyph (e.g. the `▲`/`▼` arrows in the network/disk
/// panels, or the bracket highlighting in the header's badge cluster).
pub fn draw_str_with<F: FnMut(char) -> Style>(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_x: u16,
    s: &str,
    mut style_fn: F,
) -> u16 {
    let mut cur_x = x;
    for ch in s.chars() {
        if cur_x >= max_x {
            break;
        }
        buf[(cur_x, y)].set_char(ch).set_style(style_fn(ch));
        cur_x += 1;
    }
    cur_x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_fits_exactly() {
        assert_eq!(truncate("hello", 5), Cow::Borrowed("hello"));
        assert_eq!(truncate("hi", 5), Cow::Borrowed("hi"));
    }

    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(truncate("NVIDIA GeForce RTX 4090", 10), "NVIDIA Ge…");
        assert_eq!(truncate("hello world", 6), "hello…");
    }

    #[test]
    fn truncate_too_narrow_for_ellipsis_plus_content() {
        // Output length never exceeds max_chars, even at the edges where
        // there's no room for both content and an ellipsis.
        assert_eq!(truncate("hello", 3), "he…");
        assert_eq!(truncate("hello", 1), "…");
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn truncate_multibyte_safe() {
        // Cyrillic/CJK-style multi-byte chars must not panic and must be
        // counted by character, not by byte.
        let s = "日本語のGPU名前です";
        let t = truncate(s, 5);
        assert_eq!(t.chars().count(), 5);
        assert!(t.ends_with('…'));
    }
}
