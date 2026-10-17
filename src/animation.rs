//! A deterministic clock and bounded, velocity-continuous tweens. No springs.
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    OutExpo,
    InExpo,
    InOut,
    Linear,
}
impl Curve {
    fn controls(self) -> (f64, f64, f64, f64) {
        match self {
            Self::OutExpo => (0.16, 1., 0.3, 1.),
            Self::InExpo => (0.7, 0., 0.84, 0.),
            Self::InOut => (0.76, 0., 0.24, 1.),
            Self::Linear => (0., 0., 1., 1.),
        }
    }
    pub fn sample(self, x: f64) -> f64 {
        if self == Self::Linear {
            return x.clamp(0., 1.);
        }
        let (x1, y1, x2, y2) = self.controls();
        let b = |t: f64, a: f64, c: f64| {
            3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * c + t * t * t
        };
        let mut lo = 0.;
        let mut hi = 1.;
        for _ in 0..32 {
            let t = (lo + hi) / 2.;
            if b(t, x1, x2) < x { lo = t } else { hi = t }
        }
        b((lo + hi) / 2., y1, y2)
    }
    pub fn slope(self, x: f64) -> f64 {
        if self == Self::Linear {
            return 1.;
        }
        let d = 0.00001;
        let lo = (x - d).max(0.);
        let hi = (x + d).min(1.);
        (self.sample(hi) - self.sample(lo)) / (hi - lo).max(1e-9)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    pub from: f64,
    pub target: f64,
    pub start: f64,
    pub duration: f64,
    pub curve: Curve,
    velocity_correction: f64,
}
impl Tween {
    pub fn new(from: f64, target: f64, start: f64, duration: f64, curve: Curve) -> Self {
        Self {
            from,
            target,
            start,
            duration: duration.max(0.),
            curve,
            velocity_correction: 0.,
        }
    }
    pub fn sample(self, now: f64) -> (f64, f64) {
        if self.duration == 0. || now >= self.start + self.duration {
            return (self.target, 0.);
        }
        if now < self.start {
            return (self.from, 0.);
        }
        let t = ((now - self.start) / self.duration).clamp(0., 1.);
        let delta = self.target - self.from;
        let c = self.velocity_correction;
        let value =
            self.from + delta * self.curve.sample(t) + c * self.duration * t * (1. - t).powi(2);
        let velocity = delta * self.curve.slope(t) / self.duration + c * (1. - t) * (1. - 3. * t);
        (value, velocity)
    }
    pub fn active(self, now: f64) -> bool {
        now < self.start + self.duration
    }
    /// Preserve both position and velocity with a decaying Hermite correction.
    pub fn retarget(&mut self, target: f64, now: f64, duration: f64, curve: Curve) {
        let (value, velocity) = self.sample(now);
        *self = Self::new(value, target, now, duration, curve);
        if duration > 0. {
            self.velocity_correction = velocity - (target - value) * curve.slope(0.) / duration;
        }
    }
}
#[derive(Default, Debug)]
pub struct Clock {
    now: f64,
    pub slow: bool,
    pub paused: bool,
}
impl Clock {
    pub fn now(&self) -> f64 {
        self.now
    }
    pub fn advance(&mut self, seconds: f64) {
        if !self.paused {
            self.now += seconds.max(0.) / if self.slow { 10. } else { 1. };
        }
    }
    pub fn step(&mut self, seconds: f64) {
        self.now += seconds.max(0.);
    }
}
#[derive(Default)]
pub struct Scheduler {
    pub clock: Clock,
    pub reduce_motion: bool,
    tracks: BTreeMap<u64, Tween>,
    sequences: BTreeMap<u64, Sequence>,
    finished: BTreeMap<u64, f64>,
}
impl Scheduler {
    pub fn animate(&mut self, id: u64, from: f64, to: f64, duration: f64, curve: Curve) {
        let now = self.clock.now();
        let from = if self.sequences.contains_key(&id) || self.finished.contains_key(&id) {
            self.value(id, from)
        } else {
            from
        };
        self.sequences.remove(&id);
        self.finished.remove(&id);
        let duration = if self.reduce_motion { 0.1 } else { duration };
        self.tracks
            .entry(id)
            .and_modify(|t| t.retarget(to, now, duration, curve))
            .or_insert(Tween::new(from, to, now, duration, curve));
    }
    pub fn value(&self, id: u64, fallback: f64) -> f64 {
        self.tracks
            .get(&id)
            .map(|t| t.sample(self.clock.now()).0)
            .or_else(|| {
                self.sequences
                    .get(&id)
                    .map(|s| s.sample(self.clock.now()).0)
            })
            .or_else(|| self.finished.get(&id).copied())
            .unwrap_or(fallback)
    }
    pub fn active(&self) -> bool {
        self.tracks.values().any(|t| t.active(self.clock.now()))
            || self.sequences.values().any(|s| s.active(self.clock.now()))
    }
    pub fn sequence(&mut self, id: u64, from: f64, mut segments: Vec<Segment>) {
        let from = self.value(id, from);
        if self.reduce_motion {
            for s in &mut segments {
                s.delay = 0.;
                s.duration = 0.1;
            }
        }
        self.tracks.remove(&id);
        self.finished.remove(&id);
        self.sequences.insert(
            id,
            Sequence {
                from,
                start: self.clock.now(),
                segments,
            },
        );
    }
    pub fn stagger(
        &mut self,
        items: &[(u64, f64, f64)],
        duration: f64,
        interval: f64,
        curve: Curve,
    ) {
        for (index, (id, from, to)) in items.iter().enumerate() {
            self.sequence(
                *id,
                *from,
                vec![Segment {
                    target: *to,
                    delay: index as f64 * interval,
                    duration,
                    curve,
                }],
            );
        }
    }
    pub fn forget(&mut self, id: u64) {
        self.tracks.remove(&id);
        self.sequences.remove(&id);
        self.finished.remove(&id);
    }
    pub fn finish(&mut self) {
        let now = self.clock.now();
        for (id, t) in &self.tracks {
            if !t.active(now) {
                self.finished.insert(*id, t.target);
            }
        }
        for (id, s) in &self.sequences {
            if !s.active(now) {
                self.finished.insert(*id, s.sample(now).0);
            }
        }
        self.tracks.retain(|_, t| t.active(now));
        self.sequences.retain(|_, s| s.active(now));
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotate_x: f32,
    pub rotate_y: f32,
    pub rotate_z: f32,
    pub perspective: f32,
    pub opacity: f32,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            scale: 1.,
            rotate_x: 0.,
            rotate_y: 0.,
            rotate_z: 0.,
            perspective: 800.,
            opacity: 1.,
        }
    }
}
pub fn turnstile(t: f32, back: bool, exit: bool, reduced: bool) -> Transform {
    if reduced {
        return Transform {
            opacity: if exit { 1. - t } else { t },
            ..Default::default()
        };
    }
    let sign = if back { -1. } else { 1. };
    Transform {
        x: if exit { 0. } else { 80. * (1. - t) * sign },
        rotate_y: if exit {
            50. * t * sign
        } else {
            -50. * (1. - t) * sign
        },
        opacity: if exit { 1. - t } else { t },
        ..Default::default()
    }
}
pub fn feather(now: f64, index: usize, reduced: bool) -> Transform {
    let t = Curve::OutExpo.sample(((now - index.min(7) as f64 * 0.04) / 0.3).clamp(0., 1.)) as f32;
    if reduced {
        Transform {
            opacity: (now / 0.1).clamp(0., 1.) as f32,
            ..Default::default()
        }
    } else {
        Transform {
            x: (if index == 0 { 100. } else { 60. }) * (1. - t),
            rotate_y: -35. * (1. - t),
            opacity: t,
            ..Default::default()
        }
    }
}
pub fn continuum(
    from: crate::Rect,
    to: crate::Rect,
    elapsed: f64,
    reduced: bool,
) -> (crate::Rect, f32) {
    if reduced {
        (to, (elapsed / 0.1).clamp(0., 1.) as f32)
    } else {
        (
            from.lerp(
                to,
                Curve::InOut.sample((elapsed / 0.28).clamp(0., 1.)) as f32,
            ),
            1.,
        )
    }
}
pub fn tilt(point: crate::Point, bounds: crate::Rect, amount: f32) -> Transform {
    let x = ((point.x - bounds.center().x) / (bounds.w / 2.).max(1.)).clamp(-1., 1.);
    let y = ((point.y - bounds.center().y) / (bounds.h / 2.).max(1.)).clamp(-1., 1.);
    Transform {
        rotate_x: -y * 6. * amount,
        rotate_y: x * 6. * amount,
        scale: 1. - 0.03 * amount,
        perspective: 400.,
        ..Default::default()
    }
}
/// Momentum uses elapsed time, so friction is independent of display refresh.
#[derive(Clone, Copy, Debug, Default)]
pub struct Scroll {
    pub offset: f32,
    pub velocity: f32,
    pub max: f32,
}
impl Scroll {
    pub fn tick(&mut self, dt: f32) -> bool {
        let old = self.offset;
        if self.offset < 0. || self.offset > self.max {
            let target = self.offset.clamp(0., self.max);
            self.offset += (target - self.offset) * (1. - (-dt / 0.06).exp());
            self.velocity = 0.;
            if (self.offset - target).abs() < 0.1 {
                self.offset = target
            }
        } else {
            self.offset += self.velocity * dt;
            self.velocity *= (-1.5 * dt).exp();
            if self.velocity.abs() < 0.1 {
                self.velocity = 0.
            }
        }
        self.offset = self.offset.clamp(-40., self.max + 40.);
        (old - self.offset).abs() > 0.001 || self.velocity != 0.
    }
    pub fn drag(&mut self, delta: f32) {
        let outside = self.offset < 0. || self.offset > self.max;
        self.offset =
            (self.offset + delta * if outside { 0.25 } else { 1. }).clamp(-40., self.max + 40.);
    }
}

/// Horizontal navigation shared by panorama and app-owned pivot pages.
/// Paint the title at `title_offset` and page columns at `content_offset`.
#[derive(Clone, Debug)]
pub struct Panorama {
    pub page: usize,
    pub pages: usize,
    pub width: f32,
    tween: Tween,
}
impl Panorama {
    pub fn new(pages: usize, width: f32, now: f64) -> Self {
        assert!(pages > 0 && width > 0.);
        Self {
            page: 0,
            pages,
            width,
            tween: Tween::new(0., 0., now, 0., Curve::OutExpo),
        }
    }
    pub fn select(&mut self, page: usize, now: f64, reduced: bool) {
        self.page = page.min(self.pages - 1);
        self.tween.retarget(
            self.page as f64 * self.width as f64,
            now,
            if reduced { 0. } else { 0.28 },
            Curve::OutExpo,
        );
    }
    pub fn drag(&mut self, delta: f32, now: f64) {
        let value = (self.tween.sample(now).0 + delta as f64)
            .clamp(0., self.width as f64 * (self.pages - 1) as f64);
        self.tween = Tween::new(value, value, now, 0., Curve::Linear);
    }
    pub fn release(&mut self, velocity: f32, now: f64, reduced: bool) -> usize {
        let position = self.tween.sample(now).0 / self.width as f64;
        let target = if velocity.abs() > 600. {
            if velocity > 0. {
                position.ceil()
            } else {
                position.floor()
            }
        } else {
            position.round()
        };
        self.select((target as usize).min(self.pages - 1), now, reduced);
        self.page
    }
    pub fn content_offset(&self, now: f64) -> f32 {
        -(self.tween.sample(now).0 as f32)
    }
    pub fn title_offset(&self, now: f64, reduced: bool) -> f32 {
        if reduced {
            0.
        } else {
            self.content_offset(now) * 0.5
        }
    }
    pub fn active(&self, now: f64) -> bool {
        self.tween.active(now)
    }
}
/// One step of a deterministic sequence; delay is relative to the prior step.
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub target: f64,
    pub delay: f64,
    pub duration: f64,
    pub curve: Curve,
}
#[derive(Clone, Debug)]
pub struct Sequence {
    pub from: f64,
    pub start: f64,
    pub segments: Vec<Segment>,
}
impl Sequence {
    pub fn sample(&self, now: f64) -> (f64, f64) {
        let mut from = self.from;
        let mut start = self.start;
        for segment in &self.segments {
            start += segment.delay.max(0.);
            let tween = Tween::new(from, segment.target, start, segment.duration, segment.curve);
            if now < start + segment.duration {
                return tween.sample(now);
            }
            from = segment.target;
            start += segment.duration.max(0.);
        }
        (from, 0.)
    }
    pub fn active(&self, now: f64) -> bool {
        now < self.start
            + self
                .segments
                .iter()
                .map(|s| s.delay.max(0.) + s.duration.max(0.))
                .sum::<f64>()
    }
}
