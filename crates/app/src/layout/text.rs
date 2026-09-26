//! Text and button extents under the default font (FiraMono: monospace, 0.6 em
//! advance, 1.2 em line height). Placement that depends on content size (the
//! desktop text column, how many rows a button bar wraps to) is computed from
//! these, and the overlap tests size every element with them.

use bevy::prelude::*;

const ADVANCE: f32 = 0.6;
const LINE_HEIGHT: f32 = 1.2;

/// Size of `text` in `font` px, wrapping each line at `max_width`.
#[must_use]
pub fn text_size(text: &str, font: f32, max_width: f32) -> Vec2 {
    let advance = ADVANCE * font;
    let per_line = (max_width / advance).floor().max(1.0);
    let mut lines = 0.0;
    let mut widest: f32 = 0.0;
    for line in text.lines() {
        let chars = line.chars().count() as f32;
        lines += (chars / per_line).ceil().max(1.0);
        widest = widest.max(chars.min(per_line) * advance);
    }
    Vec2::new(widest, lines * LINE_HEIGHT * font)
}

/// Outer size of a bordered button: a label, an optional smaller hint below it,
/// and padding.
#[cfg(any(test, feature = "camera"))]
#[must_use]
pub fn button_size(
    label: &str,
    hint: Option<(&str, f32)>,
    font: f32,
    pad: Vec2,
    border: f32,
) -> Vec2 {
    let mut content = text_size(label, font, f32::INFINITY);
    if let Some((hint, hint_font)) = hint {
        let h = text_size(hint, hint_font, f32::INFINITY);
        content = Vec2::new(content.x.max(h.x), content.y + h.y);
    }
    content + 2.0 * pad + Vec2::splat(2.0 * border)
}

/// How a wrapping row of items is aligned along each row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowAlign {
    /// Only the tests lay out a start-aligned bar (the desktop top bar).
    #[cfg_attr(not(test), allow(dead_code))]
    Start,
    Center,
}

/// Flex-wrap `items` into rows within `[left, left + width)`. Returns one rect
/// per row, with y measured from the container's top.
#[must_use]
pub fn wrap_rows(items: &[Vec2], left: f32, width: f32, gap: Vec2, align: RowAlign) -> Vec<Rect> {
    let mut rows: Vec<Vec<Vec2>> = Vec::new();
    for &item in items {
        let fits = rows.last().is_some_and(|row| {
            let used: f32 = row.iter().map(|b| b.x + gap.x).sum();
            used + item.x <= width
        });
        match rows.last_mut() {
            Some(row) if fits => row.push(item),
            _ => rows.push(vec![item]),
        }
    }
    let mut y = 0.0;
    rows.iter()
        .map(|row| {
            let w: f32 = row.iter().map(|b| b.x).sum::<f32>() + gap.x * (row.len() - 1) as f32;
            let h = row.iter().map(|b| b.y).fold(0.0, f32::max);
            let x = match align {
                RowAlign::Start => left,
                RowAlign::Center => left + (width - w) / 2.0,
            };
            let rect = Rect::new(x, y, x + w, y + h);
            y += h + gap.y;
            rect
        })
        .collect()
}

/// Total height of wrapped rows (0 when empty).
#[must_use]
pub fn rows_height(rows: &[Rect]) -> f32 {
    rows.last().map_or(0.0, |r| r.max.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_wraps_at_the_max_width() {
        // 10 chars at 10px font = 60px; a 30px limit fits 5 chars per line.
        let one = text_size("abcdefghij", 10.0, f32::INFINITY);
        assert_eq!(one, Vec2::new(60.0, 12.0));
        let wrapped = text_size("abcdefghij", 10.0, 30.0);
        assert_eq!(wrapped, Vec2::new(30.0, 24.0));
    }

    #[test]
    fn rows_wrap_when_the_next_item_does_not_fit() {
        let items = [Vec2::new(40.0, 10.0); 3];
        let rows = wrap_rows(&items, 0.0, 100.0, Vec2::new(10.0, 5.0), RowAlign::Start);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], Rect::new(0.0, 0.0, 90.0, 10.0));
        assert_eq!(rows[1], Rect::new(0.0, 15.0, 40.0, 25.0));
        assert_eq!(rows_height(&rows), 25.0);
    }

    #[test]
    fn centered_rows_split_the_slack() {
        let rows = wrap_rows(
            &[Vec2::new(40.0, 10.0)],
            0.0,
            100.0,
            Vec2::ZERO,
            RowAlign::Center,
        );
        assert_eq!(rows[0].min.x, 30.0);
    }
}
