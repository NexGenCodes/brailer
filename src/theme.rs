use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
    pub type_scale: TypeScale,
    pub space: Space,
    pub radius: Radius,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg: String,
    pub surface: String,
    pub ink: String,
    pub muted: String,
    pub accent: String,
    pub accent_soft: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScale {
    pub display: f32,
    pub h1: f32,
    pub h2: f32,
    pub h3: f32,
    pub body: f32,
    pub small: f32,
    pub label: f32,
    pub leading: f32,
    pub measure: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub base: f32,
    pub section: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextRole {
    Display,
    H1,
    H2,
    H3,
    Body,
    Small,
    Label,
}

impl TextRole {
    pub fn all() -> [TextRole; 7] {
        [
            TextRole::Display,
            TextRole::H1,
            TextRole::H2,
            TextRole::H3,
            TextRole::Body,
            TextRole::Small,
            TextRole::Label,
        ]
    }
}

impl Theme {
    pub fn size(&self, role: TextRole) -> f32 {
        let s = &self.type_scale;
        match role {
            TextRole::Display => s.display,
            TextRole::H1 => s.h1,
            TextRole::H2 => s.h2,
            TextRole::H3 => s.h3,
            TextRole::Body => s.body,
            TextRole::Small => s.small,
            TextRole::Label => s.label,
        }
    }

    pub fn leading(&self, role: TextRole) -> f32 {
        let s = &self.type_scale;
        match role {
            TextRole::Display => s.display * s.leading * 0.86,
            TextRole::H1 => s.h1 * s.leading * 0.9,
            TextRole::H2 => s.h2 * s.leading,
            TextRole::H3 => s.h3 * s.leading * 1.1,
            TextRole::Body => s.body * s.leading * 1.25,
            TextRole::Small => s.small * s.leading * 1.25,
            TextRole::Label => s.label * s.leading * 1.1,
        }
    }

    pub fn ink_for(&self, role: TextRole) -> &str {
        match role {
            TextRole::Display | TextRole::H1 | TextRole::H2 | TextRole::H3 => &self.palette.ink,
            TextRole::Body | TextRole::Small => &self.palette.muted,
            TextRole::Label => &self.palette.accent,
        }
    }

    pub fn family_for(&self, role: TextRole) -> &'static str {
        match role {
            TextRole::Display | TextRole::H1 | TextRole::H2 => "serif",
            _ => "sans",
        }
    }
}

pub fn builtin(name: &str) -> Option<Theme> {
    match name {
        // Warm paper + graphite + canary + band red. Dense marketplace scale:
        // smaller type, tighter leading, more products per screen.
        "canary" => Some(Theme {
            name: "canary".into(),
            palette: Palette {
                bg: "#FBF9F3".into(),
                surface: "#FFFFFF".into(),
                ink: "#17161A".into(),
                muted: "#655F55".into(),
                accent: "#B3261E".into(),
                accent_soft: "#F7D24B".into(),
                line: "#CFC7B4".into(),
            },
            type_scale: TypeScale {
                display: 64.0,
                h1: 40.0,
                h2: 28.0,
                h3: 19.0,
                body: 15.0,
                small: 13.0,
                label: 11.0,
                leading: 1.3,
                measure: 560.0,
            },
            space: Space {
                base: 8.0,
                section: 72.0,
            },
            radius: Radius {
                sm: 4.0,
                md: 10.0,
                lg: 20.0,
            },
        }),
        // Dark storefront: same dense canary scale, inverted palette. Band-red
        // is lifted to #E55B53 so it still clears 4.5:1 on both bg and surface.
        "midnight" => Some(Theme {
            name: "midnight".into(),
            palette: Palette {
                bg: "#131317".into(),
                surface: "#1E1E25".into(),
                ink: "#F5F3EC".into(),
                muted: "#A7A29B".into(),
                accent: "#E55B53".into(),
                accent_soft: "#F7D24B".into(),
                line: "#3B3B46".into(),
            },
            type_scale: TypeScale {
                display: 64.0,
                h1: 40.0,
                h2: 28.0,
                h3: 19.0,
                body: 15.0,
                small: 13.0,
                label: 11.0,
                leading: 1.3,
                measure: 560.0,
            },
            space: Space {
                base: 8.0,
                section: 72.0,
            },
            radius: Radius {
                sm: 4.0,
                md: 10.0,
                lg: 20.0,
            },
        }),
        "editorial" => Some(Theme {
            name: "editorial".into(),
            palette: Palette {
                bg: "#FAF7F2".into(),
                surface: "#EFE7DB".into(),
                ink: "#14110F".into(),
                muted: "#6B6259".into(),
                accent: "#A0492B".into(),
                accent_soft: "#EADFD2".into(),
                line: "#CEC2AE".into(),
            },
            type_scale: TypeScale {
                display: 76.0,
                h1: 48.0,
                h2: 32.0,
                h3: 21.0,
                body: 16.0,
                small: 13.0,
                label: 12.0,
                leading: 1.32,
                measure: 620.0,
            },
            space: Space {
                base: 8.0,
                section: 96.0,
            },
            radius: Radius {
                sm: 4.0,
                md: 8.0,
                lg: 16.0,
            },
        }),
        _ => None,
    }
}
