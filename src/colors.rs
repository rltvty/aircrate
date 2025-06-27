use bevy::prelude::*;

/// Vaporwave-inspired UI color palette
pub struct AirCrateColors;

impl AirCrateColors {
    pub fn background_purple() -> Color       { Color::hsla(266.0, 0.51, 0.20, 1.0) }
    pub fn gradient_magenta_start() -> Color  { Color::hsla(316.0, 0.50, 0.55, 1.0) }
    pub fn gradient_magenta_end() -> Color    { Color::hsla(330.0, 1.00, 0.75, 1.0) }
    pub fn dark_blue_ui_panel() -> Color      { Color::hsla(254.0, 0.53, 0.16, 1.0) }
    pub fn cassette_orange() -> Color         { Color::hsla(38.0,  0.89, 0.61, 1.0) }
    pub fn cassette_purple_body() -> Color    { Color::hsla(288.0, 0.42, 0.30, 1.0) }
    pub fn highlight_neon_blue() -> Color     { Color::hsla(194.0, 0.84, 0.59, 1.0) }
    pub fn thumbs_up_blue() -> Color          { Color::hsla(205.0, 0.92, 0.70, 1.0) }
    pub fn thumbs_down_red() -> Color         { Color::hsla(350.0, 0.73, 0.61, 1.0) }
    pub fn double_up_pink() -> Color          { Color::hsla(329.0, 0.66, 0.68, 1.0) }
    pub fn light_text() -> Color              { Color::hsla(0.0,   0.00, 0.80, 1.0) }
    pub fn primary_text() -> Color            { Color::hsla(258.0, 0.44, 0.93, 1.0) }
    pub fn border_lines() -> Color            { Color::hsla(267.0, 0.29, 0.38, 1.0) }
}
