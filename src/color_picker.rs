#![doc = include_str!("../docs/color-picker.md")]
use crate::{
    Point, Rect,
    icons::Icon,
    scene::{Primitive, Scene},
    theme::{Color, Theme, TypeRole},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorPickerVariant {
    #[default]
    Ribbon,
    Swatches,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompanionCount {
    #[default]
    None,
    One,
    Two,
    Three,
}
impl CompanionCount {
    pub fn get(self) -> usize {
        match self {
            Self::None => 0,
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
        }
    }
}
/// A named primary colour and up to three optional, independently chosen accents.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorScheme {
    pub name: String,
    pub primary: Color,
    pub companions: [Option<Color>; 3],
}
impl ColorScheme {
    pub fn new(name: impl Into<String>, primary: Color) -> Self {
        Self {
            name: name.into(),
            primary,
            companions: [None; 3],
        }
    }
    pub fn companions(mut self, colors: &[Color]) -> Self {
        assert!(
            colors.len() <= 3,
            "a colour scheme supports at most three companions"
        );
        self.companions = std::array::from_fn(|i| colors.get(i).copied());
        self
    }
    pub fn description(&self) -> String {
        format!("{} {}", self.name, hex(self.primary))
    }
}
/// Reusable configuration. Keep scheme order stable during interaction.
#[derive(Clone, Debug)]
pub struct ColorPicker {
    pub schemes: Vec<ColorScheme>,
    pub orientation: Orientation,
    pub variant: ColorPickerVariant,
    pub companions: CompanionCount,
}
pub(crate) struct PickerPaint<'a> {
    pub bounds: Rect,
    pub clip: Rect,
    pub label: &'a str,
    pub position: f64,
    pub selected: usize,
    pub theme: &'a Theme,
    pub disabled: bool,
}
impl ColorPicker {
    pub fn new(schemes: Vec<ColorScheme>) -> Self {
        Self {
            schemes,
            orientation: Orientation::Horizontal,
            variant: ColorPickerVariant::Ribbon,
            companions: CompanionCount::None,
        }
    }
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn variant(mut self, variant: ColorPickerVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn companions(mut self, companions: CompanionCount) -> Self {
        self.companions = companions;
        self
    }
    /// A Brockmann palette with authored light, deep and contrasting companions.
    /// This is an interpretation of the ribbon, not a Windows palette import.
    pub fn metro() -> Self {
        let colors = [
            ("yellow", 0xe3c800, 0xfff29b, 0x806d00, 0x5133ab),
            ("lime", 0xa4c400, 0xd9e88d, 0x536a00, 0x7a3db8),
            ("green", 0x60a917, 0xc3e4a3, 0x2d5c09, 0xec6d46),
            ("emerald", 0x008a00, 0x8edb9a, 0x004b25, 0xf0a14b),
            ("teal", 0x00aba9, 0x92e4dc, 0x005b60, 0xef7548),
            ("cyan", 0x1ba1e2, 0xa1dcf5, 0x005b82, 0xf5c34c),
            ("cobalt", 0x0050ef, 0xa5c3ff, 0x002e80, 0xf5c34c),
            ("indigo", 0x6a00ff, 0xc6a7ff, 0x360080, 0xb4dc55),
            ("violet", 0xaa00ff, 0xdea7ff, 0x590080, 0x80d0a2),
            ("magenta", 0xd80073, 0xffacd6, 0x79003f, 0x73d1c5),
            ("rose", 0xf472b6, 0xffd4e8, 0x9b245f, 0x51b9c0),
            ("crimson", 0xe51400, 0xffada0, 0x800d00, 0x65c8d0),
            ("orange", 0xfa6800, 0xffce9f, 0x8a3500, 0x65addb),
            ("amber", 0xf0a30a, 0xffe2a0, 0x865400, 0x7389cf),
            ("sienna", 0xa0522d, 0xdebda4, 0x552b18, 0x66b9b2),
            ("steel", 0x647687, 0xc5d1dd, 0x303d4a, 0xe9ae68),
        ];
        Self::new(
            colors
                .into_iter()
                .map(|(name, main, a, b, c)| {
                    ColorScheme::new(name, Color::hex(main)).companions(&[
                        Color::hex(a),
                        Color::hex(b),
                        Color::hex(c),
                    ])
                })
                .collect(),
        )
    }
    /// Dense Windows 8 style spectrum. Companion colours remain application data.
    pub fn windows8() -> Self {
        let colors = [
            ("gold", 0xc6b324),
            ("olive", 0x718c31),
            ("slate blue", 0x375c9c),
            ("wine", 0x8e243d),
            ("plum", 0x682d73),
            ("brick", 0x9c332c),
            ("rust", 0xb94b18),
            ("ruby", 0xbe2044),
            ("berry", 0x8c147c),
            ("indigo", 0x48249a),
            ("navy", 0x183c78),
            ("ocean", 0x16739d),
            ("forest", 0x13834b),
            ("green", 0x38a029),
            ("lime", 0x88bf24),
            ("tangerine", 0xf89619),
            ("coral", 0xee5035),
            ("fuchsia", 0xde3986),
            ("purple", 0x9c42cb),
            ("cobalt", 0x4266cb),
            ("sky", 0x239bd5),
            ("cyan", 0x2bb7d0),
            ("turquoise", 0x34c5be),
            ("mint", 0x70c391),
            ("chartreuse", 0xc4d53b),
            ("yellow", 0xf2c627),
            ("orange", 0xe99a32),
            ("salmon", 0xdb7465),
            ("orchid", 0xc87ba9),
            ("periwinkle", 0x8a90d0),
            ("sea", 0x4eaaa6),
            ("terracotta", 0xc56143),
        ];
        Self::new(
            colors
                .into_iter()
                .map(|(name, hex)| {
                    let primary = Color::hex(hex);
                    ColorScheme::new(name, primary).companions(&[
                        primary.mix(Color::hex(0xffffff), 0.50),
                        primary.mix(Color::hex(0), 0.45),
                        Color([primary.0[2], primary.0[0], primary.0[1], 1.]),
                    ])
                })
                .collect(),
        )
    }
    /// Ribbon columns are a dense legacy-style 24 px; swatches retain 48 px targets.
    pub fn cell_extent(&self) -> f32 {
        if self.variant == ColorPickerVariant::Ribbon {
            24.
        } else {
            48.
        }
    }
    pub fn selected(&self, selected: usize) -> Option<&ColorScheme> {
        self.schemes
            .get(selected.min(self.schemes.len().saturating_sub(1)))
    }
    /// Overflow follows the animated selection. Use swatches for roomy touch targets.
    pub fn cell_bounds(&self, bounds: Rect, position: f64, index: usize) -> Rect {
        let vertical = self.orientation == Orientation::Vertical;
        let inset = if self.variant == ColorPickerVariant::Ribbon {
            2.
        } else {
            0.
        };
        let extent = if vertical {
            (bounds.h - 24. - inset * 2.).max(0.)
        } else {
            (bounds.w - inset * 2.).max(0.)
        };
        let step = self.cell_extent();
        let total = self.schemes.len() as f32 * step;
        let offset =
            (position as f32 * step + step / 2. - extent / 2.).clamp(0., (total - extent).max(0.));
        let along = index as f32 * step - offset;
        if vertical {
            Rect::new(bounds.x, bounds.y + 24. + inset + along, bounds.w, step)
        } else {
            Rect::new(
                bounds.x + inset + along,
                bounds.y + 24.,
                step,
                (bounds.h - 24.).max(0.),
            )
        }
    }
    pub fn hit(&self, bounds: Rect, position: f64, point: Point) -> Option<usize> {
        if !bounds.contains(point) || point.y < bounds.y + 24. {
            return None;
        }
        (0..self.schemes.len()).find(|i| self.cell_bounds(bounds, position, *i).contains(point))
    }
    pub(crate) fn paint(&self, scene: &mut Scene, paint: PickerPaint<'_>) {
        let PickerPaint {
            bounds,
            clip,
            label,
            position,
            selected,
            theme,
            disabled,
        } = paint;
        let clip = clip.intersect(bounds);
        let fg = if disabled { theme.fg3 } else { theme.fg };
        let title = self
            .selected(selected)
            .map(|s| format!("{label} / {}", s.description()))
            .unwrap_or_else(|| format!("{label} / no colours"));
        scene.text(
            Rect::new(bounds.x, bounds.y, bounds.w, 20.),
            title,
            TypeRole::Label,
            fg,
            clip,
        );
        let strip_clip = clip.intersect(Rect::new(
            bounds.x,
            bounds.y + 24.,
            bounds.w,
            (bounds.h - 24.).max(0.),
        ));
        let vertical = self.orientation == Orientation::Vertical;
        let ribbon = self.variant == ColorPickerVariant::Ribbon;
        for (i, scheme) in self.schemes.iter().enumerate() {
            let cell = self.cell_bounds(bounds, position, i);
            let visible = cell.intersect(strip_clip);
            if visible.w <= 0. || visible.h <= 0. {
                continue;
            }
            let muted = |color: Color| {
                if disabled {
                    color.mix(theme.bg, 0.65)
                } else {
                    color
                }
            };
            if ribbon {
                let band = if vertical {
                    Rect::new(cell.x + 24., cell.y, 20., cell.h)
                } else {
                    Rect::new(cell.x, cell.y + 24., cell.w, 20.)
                };
                scene.rect(
                    band,
                    muted(scheme.primary.mix(Color::hex(0), 0.62)),
                    strip_clip,
                );
                let square = if vertical {
                    Rect::new(band.x + 6., cell.center().y - 4., 8., 8.)
                } else {
                    Rect::new(cell.center().x - 4., band.y + 6., 8., 8.)
                };
                scene.rect(square, muted(scheme.primary), strip_clip);
                for (j, color) in scheme
                    .companions
                    .iter()
                    .take(self.companions.get())
                    .enumerate()
                {
                    if let Some(color) = color {
                        let square = if vertical {
                            Rect::new(
                                band.right() + 6. + j as f32 * 8.,
                                cell.center().y - 3.,
                                6.,
                                6.,
                            )
                        } else {
                            Rect::new(cell.x + 2. + j as f32 * 8., band.y - 10., 6., 6.)
                        };
                        scene.rect(square, muted(*color), strip_clip);
                    }
                }
            } else {
                let main = if vertical {
                    Rect::new(cell.x + 12., cell.y + 8., 32., 32.)
                } else {
                    Rect::new(cell.x + 8., cell.y + 28., 32., 32.)
                };
                scene.rect(main, muted(scheme.primary), strip_clip);
                scene.stroke(main, 1., theme.line, strip_clip);
                for (j, color) in scheme
                    .companions
                    .iter()
                    .take(self.companions.get())
                    .enumerate()
                {
                    if let Some(color) = color {
                        let square = if vertical {
                            Rect::new(cell.x + 52. + j as f32 * 16., cell.y + 18., 12., 12.)
                        } else {
                            Rect::new(cell.x + 4. + j as f32 * 14., cell.y + 12., 12., 12.)
                        };
                        scene.rect(square, muted(*color), strip_clip);
                    }
                }
            }
        }
        if ribbon && !self.schemes.is_empty() {
            let first = self.cell_bounds(bounds, position, 0);
            let last = self.cell_bounds(bounds, position, self.schemes.len() - 1);
            let frame = if vertical {
                Rect::new(
                    first.x + 22.,
                    first.y - 2.,
                    24.,
                    last.bottom() - first.y + 4.,
                )
            } else {
                Rect::new(
                    first.x - 2.,
                    first.y + 22.,
                    last.right() - first.x + 4.,
                    24.,
                )
            };
            scene.stroke(frame, 1., theme.line_strong, strip_clip);
        }
        if self.schemes.is_empty() {
            return;
        }
        let position = position.clamp(0., (self.schemes.len() - 1) as f64);
        let a = position.floor() as usize;
        let b = position.ceil() as usize;
        let cell = self.cell_bounds(bounds, position, a).lerp(
            self.cell_bounds(bounds, position, b),
            position.fract() as f32,
        );
        if ribbon {
            let triangles = if vertical {
                let right = if self.companions == CompanionCount::None {
                    cell.x + 50.
                } else {
                    cell.x + 54. + self.companions.get() as f32 * 8.
                };
                [
                    [
                        Point::new(cell.x + 10., cell.center().y - 4.),
                        Point::new(cell.x + 10., cell.center().y + 4.),
                        Point::new(cell.x + 17., cell.center().y),
                    ],
                    [
                        Point::new(right + 7., cell.center().y - 4.),
                        Point::new(right + 7., cell.center().y + 4.),
                        Point::new(right, cell.center().y),
                    ],
                ]
            } else {
                let top = cell.y
                    + if self.companions == CompanionCount::None {
                        10.
                    } else {
                        0.
                    };
                [
                    [
                        Point::new(cell.center().x - 4., top),
                        Point::new(cell.center().x + 4., top),
                        Point::new(cell.center().x, top + 7.),
                    ],
                    [
                        Point::new(cell.center().x - 4., cell.y + 58.),
                        Point::new(cell.center().x + 4., cell.y + 58.),
                        Point::new(cell.center().x, cell.y + 51.),
                    ],
                ]
            };
            for points in triangles {
                for i in 0..3 {
                    scene.push(
                        Primitive::Line {
                            from: points[i],
                            to: points[(i + 1) % 3],
                            width: 1.,
                            color: fg,
                        },
                        strip_clip,
                    );
                }
            }
        } else if vertical {
            Icon::CaretRight.paint(
                scene,
                Rect::new(cell.x, cell.y + 18., 12., 12.),
                fg,
                strip_clip,
            );
            let x = if self.companions == CompanionCount::None {
                cell.x + 46.
            } else {
                cell.x + 52. + self.companions.get() as f32 * 16.
            };
            Icon::CaretLeft.paint(scene, Rect::new(x, cell.y + 18., 12., 12.), fg, strip_clip);
        } else {
            Icon::CaretDown.paint(
                scene,
                Rect::new(cell.x + 18., cell.y - 1., 12., 12.),
                fg,
                strip_clip,
            );
            Icon::CaretUp.paint(
                scene,
                Rect::new(cell.x + 18., cell.y + 62., 12., 12.),
                fg,
                strip_clip,
            );
        }
    }
}
pub fn hex(color: Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (color.0[0].clamp(0., 1.) * 255.).round() as u8,
        (color.0[1].clamp(0., 1.) * 255.).round() as u8,
        (color.0[2].clamp(0., 1.) * 255.).round() as u8
    )
}
