pub mod fonts;
pub mod layout;
pub mod primitive;
pub mod render;
pub mod spec;
pub mod svg;
pub mod theme;
pub mod verify;

use anyhow::{Result, bail};
use std::path::Path;

pub use fonts::FontBook;
pub use primitive::Scene;
pub use spec::{Document, Frame};
pub use theme::Theme;

pub struct Pipeline {
    pub theme: Theme,
    pub fonts: FontBook,
}

impl Pipeline {
    pub fn new(theme_name: &str) -> Result<Pipeline> {
        let theme = match theme::builtin(theme_name) {
            Some(t) => t,
            None => bail!(
                "unknown theme '{theme_name}' (available: {})",
                known_themes().join(", ")
            ),
        };
        let fonts = FontBook::discover(&FontBook::default_roots());
        Ok(Pipeline { theme, fonts })
    }

    pub fn scene(&self, frame: &Frame) -> Scene {
        let ctx = layout::Ctx {
            theme: &self.theme,
            fonts: &self.fonts,
        };
        layout::build(frame, &ctx)
    }

    pub fn verify(&self, scene: &Scene) -> verify::Report {
        let mut rep = verify::scene(scene, 0.75);
        rep.contrast.extend(verify::theme(&self.theme));
        rep
    }

    pub fn write_svg(&self, path: &Path, scene: &Scene) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, crate::svg::render(scene))?;
        Ok(())
    }

    pub fn write_png(&self, path: &Path, scene: &Scene, scale: f32) -> Result<()> {
        render::write_png(path, scene, scale, &self.fonts)
    }
}

pub fn load(path: &Path) -> Result<Document> {
    let raw = std::fs::read_to_string(path)?;
    let mut doc: Document = serde_json::from_str(&raw)?;
    let base = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let mut errors = Vec::new();
    spec::expand_components(&mut doc, &mut errors);
    spec::resolve_images(&mut doc, &base, &mut errors);
    errors.extend(spec::validate(&doc));
    if !errors.is_empty() {
        bail!("invalid spec:\n  {}", errors.join("\n  "));
    }
    Ok(doc)
}

pub fn known_themes() -> Vec<&'static str> {
    vec!["canary", "editorial", "midnight"]
}
