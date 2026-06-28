//! The "forge" palette — the single source of truth for colours across the Bevy
//! visualizer/animator, matching the web redesign tokens (see the design handoff).
//! Values are non-linear sRGB (hex / 255), the same space `Color::srgb` expects.
#![allow(dead_code)] // a complete token set — not every token is referenced yet

use bevy::prelude::Color;

/// `(r,g,b)` from 0–255 hex bytes as a `Color::srgb`.
const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::srgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}
/// …with an alpha (0–1).
const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::srgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

// surfaces
pub const BG: Color = rgb(0x0b, 0x0d, 0x11); // app background
pub const BG2: Color = rgb(0x0e, 0x11, 0x16); // recessed surfaces / sidebars
pub const PANEL: Color = rgb(0x12, 0x16, 0x1d);
pub const PANEL2: Color = rgb(0x16, 0x1b, 0x23);
pub const EDGE: Color = rgb(0x22, 0x2a, 0x34); // hairline borders
pub const EDGE2: Color = rgb(0x31, 0x3b, 0x48); // brighter borders, graph edges

// text
pub const INK: Color = rgb(0xe8, 0xeb, 0xf0);
pub const MUT: Color = rgb(0x8b, 0x94, 0xa3);
pub const FAINT: Color = rgb(0x52, 0x5b, 0x69);

// accents
pub const ACC: Color = rgb(0xff, 0x6a, 0x3a); // molten — primary / active edge / goal ring
pub const CY: Color = rgb(0x46, 0xd3, 0xc6); // cyan — solved / target / selection ring

// graph entities
pub const NODE_PURPLE: Color = rgb(0x6d, 0x5f, 0xd6); // location nodes
pub const GREY_NODE: Color = rgb(0x7e, 0x87, 0x94); // generic nodes
pub const RIG_GREEN: Color = rgb(0xa8, 0xd2, 0x4a); // rig / truck mobiles
pub const CRATE_AMBER: Color = rgb(0xe8, 0xb9, 0x4a); // package mobiles

// panel fill with the standard inspector translucency
pub const PANEL_BLUR: Color = rgba(0x0e, 0x11, 0x16, 0.92);
// translucent drop-zone fill in the block editor
pub const ZONE: Color = rgba(0x12, 0x16, 0x1d, 0.6);
