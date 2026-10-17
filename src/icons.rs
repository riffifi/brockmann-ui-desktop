//! The closed Phosphor set. Actual MIT-licensed SVG paths are tessellated at build time.
use crate::{
    Point, Rect,
    scene::{Primitive, Scene},
    theme::Color,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Icon {
    DotsThree,
    DotsThreeVertical,
    Plus,
    Minus,
    X,
    Check,
    CaretUp,
    CaretDown,
    CaretLeft,
    CaretRight,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Play,
    Stop,
    Record,
}
include!(concat!(env!("OUT_DIR"), "/icons.rs"));
impl Icon {
    pub fn paint(self, scene: &mut Scene, bounds: Rect, color: Color, clip: Rect) {
        let (vertices, indices) = MESHES[self as usize + if bounds.w <= 16. { 17 } else { 0 }];
        for triangle in indices.chunks_exact(3) {
            let p = |i: u16| {
                let v = vertices[i as usize];
                Point::new(
                    bounds.x + v[0] / 256. * bounds.w,
                    bounds.y + v[1] / 256. * bounds.h,
                )
            };
            scene.push(
                Primitive::Triangle {
                    points: [p(triangle[0]), p(triangle[1]), p(triangle[2])],
                    color,
                },
                clip,
            );
        }
    }
}
