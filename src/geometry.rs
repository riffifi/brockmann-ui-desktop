//! Logical geometry. Round edges, rather than dimensions, after device scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }
    pub fn center(self) -> Point {
        Point::new(self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(
            self.x + n,
            self.y + n,
            (self.w - 2. * n).max(0.),
            (self.h - 2. * n).max(0.),
        )
    }
    pub fn translate(self, x: f32, y: f32) -> Self {
        Self::new(self.x + x, self.y + y, self.w, self.h)
    }
    pub fn intersect(self, b: Self) -> Self {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        Self::new(
            x,
            y,
            (self.right().min(b.right()) - x).max(0.),
            (self.bottom().min(b.bottom()) - y).max(0.),
        )
    }
    pub fn snap(self, scale: f32) -> Self {
        let x = (self.x * scale).round();
        let y = (self.y * scale).round();
        Self::new(
            x,
            y,
            (self.right() * scale).round() - x,
            (self.bottom() * scale).round() - y,
        )
    }
    pub fn lerp(self, b: Self, t: f32) -> Self {
        Self::new(
            self.x + (b.x - self.x) * t,
            self.y + (b.y - self.y) * t,
            self.w + (b.w - self.w) * t,
            self.h + (b.h - self.h) * t,
        )
    }
}
