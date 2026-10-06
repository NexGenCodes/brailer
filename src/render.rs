use anyhow::{Context, Result, anyhow};

use crate::fonts::FontBook;
use crate::primitive::Scene;

pub const MAX_DIM: u32 = 16000;

pub fn fontdb(fonts: &FontBook) -> usvg::fontdb::Database {
    let mut db = usvg::fontdb::Database::new();
    for file in fonts.files() {
        let _ = db.load_font_file(&file);
    }
    db
}

pub fn check_output(width: f32, height: f32, scale: f32) -> Result<(u32, u32)> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(anyhow!("scale must be a finite value > 0, got {scale}"));
    }
    if !width.is_finite() || !height.is_finite() {
        return Err(anyhow!("scene size is not finite: {width}x{height}"));
    }
    let tw = width.round() as f64 * scale as f64;
    let th = height.round() as f64 * scale as f64;
    if tw < 1.0 || th < 1.0 {
        return Err(anyhow!("degenerate output size at scale {scale}"));
    }
    if tw > MAX_DIM as f64 || th > MAX_DIM as f64 {
        return Err(anyhow!(
            "output {tw}x{th} exceeds {MAX_DIM}px limit; lower --scale (scene is {}x{})",
            width as i64,
            height as i64
        ));
    }
    Ok((tw as u32, th as u32))
}

pub fn png(scene: &Scene, scale: f32, fonts: &FontBook) -> Result<Vec<u8>> {
    check_output(scene.width, scene.height, scale)?;
    let doc = crate::svg::render(scene);

    let opts = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb(fonts)),
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(doc.as_bytes(), &opts).context("svg parse failed")?;
    let base = tree.size().to_int_size();
    let target = base
        .scale_by(scale)
        .ok_or_else(|| anyhow!("scaled size overflow at scale {scale}"))?;
    if target.width() == 0 || target.height() == 0 {
        return Err(anyhow!("degenerate output size at scale {scale}"));
    }
    if target.width() > MAX_DIM || target.height() > MAX_DIM {
        return Err(anyhow!(
            "output {}x{} exceeds {MAX_DIM}px limit; lower --scale",
            target.width(),
            target.height()
        ));
    }

    let mut pixmap = tiny_skia::Pixmap::new(target.width(), target.height())
        .ok_or_else(|| anyhow!("could not allocate pixmap"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| anyhow!("png encode failed: {e}"))
}

pub fn write_png(
    path: &std::path::Path,
    scene: &Scene,
    scale: f32,
    fonts: &FontBook,
) -> Result<()> {
    let bytes = png(scene, scale, fonts)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, &bytes).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}
