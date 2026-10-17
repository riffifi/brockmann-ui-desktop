//! Shared primitives for built-in and application widgets. Order is painter order.
use crate::{
    Point, Rect,
    animation::Transform,
    theme::{Color, TypeRole},
};
#[derive(Clone, Debug)]
pub enum Primitive {
    ShapedText {
        bounds: Rect,
        buffer: std::sync::Arc<cosmic_text::Buffer>,
        color: Color,
    },
    Triangle {
        points: [Point; 3],
        color: Color,
    },
    Rect {
        bounds: Rect,
        color: Color,
        stroke: f32,
    },
    Arc {
        bounds: Rect,
        color: Color,
        start: f32,
        sweep: f32,
        thickness: f32,
    },
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: Color,
    },
    Text {
        bounds: Rect,
        text: String,
        role: TypeRole,
        color: Color,
        weight: Option<u16>,
    },
}
#[derive(Clone, Debug)]
pub struct Draw {
    pub primitive: Primitive,
    pub clip: Rect,
    pub transform: Transform,
    pub origin: Point,
}
#[derive(Default, Clone, Debug)]
pub struct Scene {
    pub draws: Vec<Draw>,
}
impl Scene {
    pub fn push(&mut self, p: Primitive, clip: Rect) {
        self.draws.push(Draw {
            primitive: p,
            clip,
            transform: Transform::default(),
            origin: Point::default(),
        });
    }
    pub fn rect(&mut self, r: Rect, c: Color, clip: Rect) {
        self.push(
            Primitive::Rect {
                bounds: r,
                color: c,
                stroke: 0.,
            },
            clip,
        )
    }
    pub fn stroke(&mut self, r: Rect, width: f32, c: Color, clip: Rect) {
        self.push(
            Primitive::Rect {
                bounds: r,
                color: c,
                stroke: width,
            },
            clip,
        )
    }
    pub fn text(
        &mut self,
        r: Rect,
        text: impl Into<String>,
        role: TypeRole,
        color: Color,
        clip: Rect,
    ) {
        self.push(
            Primitive::Text {
                bounds: r,
                text: text.into(),
                role,
                color,
                weight: None,
            },
            clip,
        )
    }
    pub fn arc(
        &mut self,
        r: Rect,
        start: f32,
        sweep: f32,
        thickness: f32,
        color: Color,
        clip: Rect,
    ) {
        self.push(
            Primitive::Arc {
                bounds: r,
                color,
                start,
                sweep,
                thickness,
            },
            clip,
        )
    }
    pub fn transform_since(&mut self, index: usize, t: Transform, origin: Point) {
        for d in &mut self.draws[index..] {
            d.transform = t;
            d.origin = origin;
        }
    }
}
