#![doc = include_str!("../docs/guide.md")]
//! Native, damage-driven UI with a single Metro/Swiss design language.
//!
//! Build a keyed [`Widget`] tree each frame. [`Ui::reconcile`] preserves interaction
//! and animation state for surviving keys; an application owns its domain state.
//! All geometry is in logical pixels. [`Renderer`] performs physical pixel snapping.
pub mod animation;
pub mod color_picker;
pub mod geometry;
pub mod icons;
pub mod layout;
pub mod render;
pub mod runtime;
pub mod scene;
pub mod text;
pub mod theme;
pub mod ui;
pub mod widget;
pub use color_picker::{ColorPicker, ColorPickerVariant, ColorScheme, CompanionCount, Orientation};
pub use geometry::{Point, Rect};
pub use render::Renderer;
pub use runtime::{Application, InputSender, run};
pub use theme::{Accent, Theme};
pub use ui::{Event, Input, Ui};
pub use widget::{Id, Widget};
