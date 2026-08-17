//! Material 3 colour tokens

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A packed RGBA colour (`0xRRGGBBAA`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u32);

impl Rgba {
    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa`
    pub fn from_hex(input: &str) -> Option<Self> {
        let raw = input.trim().trim_start_matches('#');
        match raw.len() {
            3 => {
                let mut hex = String::with_capacity(8);
                for c in raw.chars() {
                    hex.extend([c, c]);
                }
                hex.push_str("ff");
                u32::from_str_radix(&hex, 16).ok().map(Self)
            }
            6 => u32::from_str_radix(raw, 16)
                .ok()
                .map(|v| Self((v << 8) | 0xff)),
            8 => u32::from_str_radix(raw, 16).ok().map(Self),
            _ => None,
        }
    }

    /// CSS form `#rrggbb`
    pub fn to_css(self) -> String {
        if self.0 & 0xff == 0xff {
            format!("#{:06x}", self.0 >> 8)
        } else {
            format!("#{:08x}", self.0)
        }
    }

    pub fn luminance(self) -> u32 {
        let r = self.0 >> 24;
        let g = (self.0 >> 16) & 0xff;
        let b = (self.0 >> 8) & 0xff;
        (r * 299 + g * 587 + b * 114) / 1000
    }

    pub fn dim(self, factor: f64) -> Self {
        let r = ((self.0 >> 24) as f64 * factor).clamp(0.0, 255.0) as u32;
        let g = (((self.0 >> 16) & 0xff) as f64 * factor).clamp(0.0, 255.0) as u32;
        let b = (((self.0 >> 8) & 0xff) as f64 * factor).clamp(0.0, 255.0) as u32;
        Self(r << 24 | g << 16 | b << 8 | (self.0 & 0xff))
    }
}

impl std::fmt::Display for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_css())
    }
}

impl Serialize for Rgba {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.to_css())
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::from_hex(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid colour {raw:?}")))
    }
}

/// Normalize a token name: lowercase, alphanumerics only
pub fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

macro_rules! rgba {
    ($hex:literal) => {
        Rgba::from_hex($hex).expect("literal colour is valid")
    };
}

macro_rules! palette {
    ($($name:ident),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Palette {
            $(pub $name: Rgba),+
        }

        impl Palette {
            /// `token name -> css colour` over every token
            pub fn tokens(&self) -> HashMap<String, String> {
                let mut map = HashMap::new();
                $(map.insert(stringify!($name).to_string(), self.$name.to_css());)+
                map
            }

            /// Every token as `(name, css colour)` in declaration order
            pub fn entries(&self) -> Vec<(&'static str, String)> {
                vec![$( (stringify!($name), self.$name.to_css()) ),+]
            }

            /// Overlay values from an arbitrary key/value map
            pub fn from_token_map(&mut self, map: &HashMap<String, String>) {
                $(
                    if let Some(value) = map.get(&normalize(stringify!($name))) {
                        if let Some(colour) = Rgba::from_hex(value) {
                            self.$name = colour;
                        }
                    }
                )+
            }
        }
    };
}

palette!(
    primary,
    on_primary,
    primary_container,
    on_primary_container,
    secondary,
    on_secondary,
    secondary_container,
    on_secondary_container,
    tertiary,
    on_tertiary,
    tertiary_container,
    on_tertiary_container,
    error,
    on_error,
    error_container,
    on_error_container,
    background,
    on_background,
    surface,
    on_surface,
    surface_variant,
    on_surface_variant,
    outline,
    outline_variant,
    shadow,
    scrim,
    inverse_surface,
    inverse_on_surface,
    inverse_primary,
    surface_dim,
    surface_bright,
    surface_container_lowest,
    surface_container_low,
    surface_container,
    surface_container_high,
    surface_container_highest,
);

/// Neutral fallback used when no generator is reachable or the wallpaper is missing
impl Default for Palette {
    fn default() -> Self {
        Self {
            primary: rgba!("#5b5cf0"),
            on_primary: rgba!("#ffffff"),
            primary_container: rgba!("#e0e0ff"),
            on_primary_container: rgba!("#11055b"),
            secondary: rgba!("#5b5d72"),
            on_secondary: rgba!("#ffffff"),
            secondary_container: rgba!("#e0e1f9"),
            on_secondary_container: rgba!("#181a2c"),
            tertiary: rgba!("#76546d"),
            on_tertiary: rgba!("#ffffff"),
            tertiary_container: rgba!("#ffd7f0"),
            on_tertiary_container: rgba!("#2c1226"),
            error: rgba!("#b3261e"),
            on_error: rgba!("#ffffff"),
            error_container: rgba!("#f9dedc"),
            on_error_container: rgba!("#410e0b"),
            background: rgba!("#f6f5ff"),
            on_background: rgba!("#1a1b21"),
            surface: rgba!("#f6f5ff"),
            on_surface: rgba!("#1a1b21"),
            surface_variant: rgba!("#e2e1ec"),
            on_surface_variant: rgba!("#45464f"),
            outline: rgba!("#767680"),
            outline_variant: rgba!("#c6c5d0"),
            shadow: rgba!("#000000"),
            scrim: rgba!("#000000"),
            inverse_surface: rgba!("#2f3036"),
            inverse_on_surface: rgba!("#f1f0f7"),
            inverse_primary: rgba!("#bec3ff"),
            surface_dim: rgba!("#d6d5e0"),
            surface_bright: rgba!("#f6f5ff"),
            surface_container_lowest: rgba!("#ffffff"),
            surface_container_low: rgba!("#f0effa"),
            surface_container: rgba!("#ebeff4"),
            surface_container_high: rgba!("#e5e4ee"),
            surface_container_highest: rgba!("#dfdee9"),
        }
    }
}

impl Palette {
    /// Whether a palette is a dark scheme, i.e. dark surfaces with light text
    pub fn is_dark(&self) -> bool {
        self.surface.luminance() < self.on_surface.luminance()
    }
}
