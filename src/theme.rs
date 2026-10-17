//! Achromatic tokens and contrast-safe accents. Color components are sRGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub [f32; 4]);
impl Color {
    pub const fn hex(n: u32) -> Self {
        Self([
            ((n >> 16) & 255) as f32 / 255.,
            ((n >> 8) & 255) as f32 / 255.,
            (n & 255) as f32 / 255.,
            1.,
        ])
    }
    pub fn alpha(self, a: f32) -> Self {
        Self([self.0[0], self.0[1], self.0[2], self.0[3] * a])
    }
    pub fn mix(self, b: Self, t: f32) -> Self {
        Self(std::array::from_fn(|i| {
            self.0[i] + (b.0[i] - self.0[i]) * t
        }))
    }
    pub fn linear(self) -> [f32; 4] {
        let mut v = self.0;
        for x in &mut v[..3] {
            *x = if *x <= 0.04045 {
                *x / 12.92
            } else {
                ((*x + 0.055) / 1.055).powf(2.4)
            };
        }
        v
    }
    pub fn luminance(self) -> f32 {
        let c = self.linear();
        c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
    }
    pub fn contrast(self, b: Self) -> f32 {
        let a = self.luminance();
        let b = b.luminance();
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accent {
    #[default]
    Teal,
    Cobalt,
    Lime,
    Orange,
    Crimson,
    Magenta,
    Violet,
}
impl Accent {
    pub const ALL: [Self; 7] = [
        Self::Teal,
        Self::Cobalt,
        Self::Lime,
        Self::Orange,
        Self::Crimson,
        Self::Magenta,
        Self::Violet,
    ];
    pub fn color(self) -> Color {
        Color::hex(match self {
            Self::Teal => 0x00aba9,
            Self::Cobalt => 0x0050ef,
            Self::Lime => 0xa4c400,
            Self::Orange => 0xfa6800,
            Self::Crimson => 0xe51400,
            Self::Magenta => 0xd80073,
            Self::Violet => 0xaa00ff,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Teal => "teal",
            Self::Cobalt => "cobalt",
            Self::Lime => "lime",
            Self::Orange => "orange",
            Self::Crimson => "crimson",
            Self::Magenta => "magenta",
            Self::Violet => "violet",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub light: bool,
    pub bg: Color,
    pub surface: Color,
    pub fg: Color,
    pub fg2: Color,
    pub fg3: Color,
    pub line: Color,
    pub line_strong: Color,
    pub accent: Color,
    pub accent_press: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub text_scale: f32,
    pub reduce_motion: bool,
    pub tilt: bool,
}
impl Default for Theme {
    fn default() -> Self {
        Self::new(false, Accent::Teal)
    }
}
impl Theme {
    pub fn new(light: bool, accent: Accent) -> Self {
        let c = |dark, bright| Color::hex(if light { bright } else { dark });
        let accent = accent.color();
        let white = Color::hex(0xffffff);
        Self {
            light,
            bg: c(0, 0xffffff),
            surface: c(0x121212, 0xf2f2f2),
            fg: c(0xffffff, 0),
            fg2: c(0xa6a6a6, 0x5a5a5a),
            fg3: c(0x5c5c5c, 0xa8a8a8),
            line: c(0x3a3a3a, 0xbdbdbd),
            line_strong: c(0x8a8a8a, 0x4a4a4a),
            accent,
            accent_press: accent.mix(Color::hex(0), 0.18),
            on_accent: if white.contrast(accent) >= 4.5 {
                white
            } else {
                Color::hex(0)
            },
            danger: Color::hex(0xe51400),
            text_scale: 1.,
            reduce_motion: false,
            tilt: true,
        }
    }
    /// Replace the single chrome accent, preserving readable text on filled controls.
    pub fn with_accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self.accent_press = accent.mix(Color::hex(0), 0.18);
        let white = Color::hex(0xffffff);
        self.on_accent = if white.contrast(accent) >= 4.5 {
            white
        } else {
            Color::hex(0)
        };
        self
    }
    pub fn mix(&self, b: &Self, t: f32) -> Self {
        let mut v = b.clone();
        macro_rules! mix { ($($f:ident),*)=>{$(v.$f=self.$f.mix(b.$f,t);)*}; }
        mix!(
            bg,
            surface,
            fg,
            fg2,
            fg3,
            line,
            line_strong,
            accent,
            accent_press,
            on_accent,
            danger
        );
        v
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRole {
    Display,
    Title,
    Heading,
    Value,
    Body,
    Label,
    Caption,
    MonoValue,
    MonoLarge,
    PixelDisplay,
    PixelCircle,
    PixelGrid,
    PixelLine,
    PixelTriangle,
}
#[derive(Clone, Copy, Debug)]
pub struct TypeStyle {
    pub family: &'static str,
    pub size: f32,
    pub line: f32,
    pub weight: u16,
    pub tracking: f32,
}
impl TypeRole {
    pub fn style(self) -> TypeStyle {
        use TypeRole::*;
        let (family, size, line, weight, tracking) = match self {
            Display => ("Geist", 72., 72., 200, -0.02),
            Title => ("Geist", 40., 44., 300, -0.01),
            Heading => ("Geist", 24., 28., 400, 0.),
            Value => ("Geist", 20., 24., 500, 0.),
            Body => ("Geist", 14., 20., 400, 0.),
            Label => ("Geist", 12., 16., 400, 0.04),
            Caption => ("Geist", 11., 16., 400, 0.02),
            MonoValue => ("Geist Mono", 14., 20., 400, 0.),
            MonoLarge => ("Geist Mono", 32., 36., 300, 0.),
            PixelCircle => ("Geist Pixel Circle", 64., 72., 400, 0.),
            PixelGrid => ("Geist Pixel Grid", 64., 72., 400, 0.),
            PixelLine => ("Geist Pixel Line", 64., 72., 400, 0.),
            PixelTriangle => ("Geist Pixel Triangle", 64., 72., 400, 0.),
            PixelDisplay => ("Geist Pixel Square", 64., 72., 400, 0.),
        };
        TypeStyle {
            family,
            size,
            line,
            weight,
            tracking,
        }
    }
}
