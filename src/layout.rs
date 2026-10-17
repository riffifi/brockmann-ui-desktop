//! Twelve-column composition, four-pixel rhythm, and Metro tiles.
use crate::geometry::Rect;
pub const UNIT: f32 = 4.;
pub fn rhythm(v: f32) -> f32 {
    (v / UNIT).round() * UNIT
}
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub bounds: Rect,
    pub margin: f32,
    pub gutter: f32,
    pub column: f32,
}
impl Grid {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let width = width / scale;
        let margin = if width < 720. {
            24.
        } else if width <= 1280. {
            48.
        } else {
            96.
        };
        let gutter = 16.;
        Self {
            bounds: Rect::new(
                margin * scale,
                0.,
                (width - 2. * margin).max(0.) * scale,
                height,
            ),
            margin: margin * scale,
            gutter: gutter * scale,
            column: ((width - 2. * margin - 11. * gutter) / 12.).max(0.) * scale,
        }
    }
    /// Columns are zero based; the second composition axis is column four.
    pub fn span(self, start: usize, count: usize) -> Rect {
        assert!(start < 12 && count > 0 && start + count <= 12);
        Rect::new(
            self.bounds.x + start as f32 * (self.column + self.gutter),
            0.,
            count as f32 * self.column + (count - 1) as f32 * self.gutter,
            self.bounds.h,
        )
    }
    pub fn tile(self, x: u32, y: u32, w: u32, h: u32) -> Rect {
        assert!([1, 2, 4].contains(&w) && [1, 2].contains(&h));
        Rect::new(
            self.margin + x as f32 * 104.,
            y as f32 * 104.,
            w as f32 * 104. - 8.,
            h as f32 * 104. - 8.,
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Layout {
    Leaf,
    Vertical { gap: f32, padding: f32 },
    Horizontal { gap: f32, padding: f32 },
    Tiles { columns: usize },
    Overlay,
}
#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: Option<f32>,
    pub height: f32,
    pub grow: f32,
}
impl Default for Size {
    fn default() -> Self {
        Self {
            width: None,
            height: 40.,
            grow: 0.,
        }
    }
}
/// Stack placement distributes the remaining space among children with grow > 0.
pub fn stack(
    bounds: Rect,
    children: &[Size],
    horizontal: bool,
    gap: f32,
    padding: f32,
) -> Vec<Rect> {
    let b = bounds.inset(padding);
    let total = if horizontal { b.w } else { b.h };
    let fixed: f32 = children
        .iter()
        .map(|c| {
            if c.grow > 0. {
                0.
            } else if horizontal {
                c.width.unwrap_or(c.height)
            } else {
                c.height
            }
        })
        .sum();
    let grows: f32 = children.iter().map(|c| c.grow).sum();
    let extra = (total - fixed - gap * children.len().saturating_sub(1) as f32).max(0.);
    let mut cursor = if horizontal { b.x } else { b.y };
    children
        .iter()
        .map(|c| {
            let n = if c.grow > 0. {
                extra * c.grow / grows
            } else if horizontal {
                c.width.unwrap_or(c.height)
            } else {
                c.height
            };
            let r = if horizontal {
                Rect::new(cursor, b.y, n, c.height.min(b.h))
            } else {
                Rect::new(b.x, cursor, c.width.unwrap_or(b.w).min(b.w), n)
            };
            cursor += n + gap;
            r
        })
        .collect()
}
