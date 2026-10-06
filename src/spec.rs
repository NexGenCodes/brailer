use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::theme::TextRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub theme: String,
    #[serde(default)]
    pub asset_dir: Option<String>,
    #[serde(default)]
    pub components: BTreeMap<String, Node>,
    // Legacy single-frame form: canvas + root at the top level. When `frames`
    // is present it takes precedence (one responsive view per entry).
    #[serde(default)]
    pub canvas: Option<Canvas>,
    #[serde(default)]
    pub root: Option<Node>,
    #[serde(default)]
    pub frames: BTreeMap<String, Frame>,
}

impl Document {
    /// The responsive frames of this spec. A legacy `canvas`+`root` doc becomes
    /// a single frame named `design` so the first version stays valid.
    pub fn frames_effective(&self) -> BTreeMap<String, Frame> {
        if !self.frames.is_empty() {
            return self.frames.clone();
        }
        let mut m = BTreeMap::new();
        if let Some(canvas) = &self.canvas {
            m.insert(
                "design".into(),
                Frame {
                    canvas: canvas.clone(),
                    root: self.root.clone().unwrap_or_default(),
                },
            );
        }
        m
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub canvas: Canvas,
    pub root: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub width: f32,
    pub background: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Start,
    Center,
    End,
}

/// A colour can be a literal hex or a two-stop linear gradient. Strings stay
/// valid, so `"bg": "#fff"` from the first version still parses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fill {
    Solid(String),
    Linear(LinearGradient),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearGradient {
    pub from: String,
    pub to: String,
    #[serde(default = "default_angle")]
    pub angle: f32,
}

fn default_angle() -> f32 {
    180.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shadow {
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_shadow_color")]
    pub color: String,
}

fn default_shadow_color() -> String {
    "#17161A24".into()
}

/// How an image is drawn inside its box. `cover` crops to fill (photos),
/// `contain` letterboxes, `fill` stretches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    #[default]
    Cover,
    Contain,
    Fill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Node {
    Stack {
        gap: Option<f32>,
        align: Option<Align>,
        pad: Option<f32>,
        bg: Option<Fill>,
        shadow: Option<Shadow>,
        children: Vec<Node>,
    },
    Grid {
        columns: u8,
        gap: Option<f32>,
        align: Option<Align>,
        pad: Option<f32>,
        bg: Option<Fill>,
        shadow: Option<Shadow>,
        children: Vec<Node>,
    },
    Text {
        text: String,
        role: TextRole,
        #[serde(rename = "align")]
        text_align: Option<Align>,
        color: Option<String>,
        max_measure: Option<f32>,
        // Tracking in ems relative to the role size: 0.04 tightens labels the
        // way a designer would `letter-spacing: 0.04em`.
        #[serde(default)]
        tracking: Option<f32>,
    },
    Rule {
        thickness: Option<f32>,
        color: Option<String>,
    },
    Spacer {
        height: f32,
    },
    Card {
        bg: Option<Fill>,
        border: Option<String>,
        radius: Option<f32>,
        pad: Option<f32>,
        shadow: Option<Shadow>,
        children: Vec<Node>,
    },
    /// A grid cell that spans `span` columns. Only valid as a direct child of
    /// `grid`. Spanning the full width is how you get a wide promo cell next to
    /// narrow product cards — the asymmetry a marketplace needs.
    Cell {
        span: u8,
        child: Box<Node>,
    },
    /// A first-class image. `src` is either a `data:` URI or a path resolved
    /// against the spec's `asset_dir`. `width`/`height` are the box; `fit`
    /// controls how the source fills it.
    Image {
        src: String,
        width: Option<f32>,
        height: f32,
        #[serde(default)]
        fit: Fit,
    },
    Raw {
        svg: String,
        height: Option<f32>,
    },
    /// Expanded to its component tree at load time. Never survives into
    /// layout; it exists so responsive frames can share a header or product
    /// card instead of duplicating it.
    Component {
        name: String,
    },
}

impl Default for Node {
    fn default() -> Node {
        Node::stack(Vec::new())
    }
}

impl Node {
    pub fn stack(children: Vec<Node>) -> Node {
        Node::Stack {
            gap: None,
            align: None,
            pad: None,
            bg: None,
            shadow: None,
            children,
        }
    }

    pub fn grid(columns: u8, children: Vec<Node>) -> Node {
        Node::Grid {
            columns,
            gap: None,
            align: None,
            pad: None,
            bg: None,
            shadow: None,
            children,
        }
    }

    pub fn text(text: &str, role: TextRole) -> Node {
        Node::Text {
            text: text.into(),
            role,
            text_align: None,
            color: None,
            max_measure: None,
            tracking: None,
        }
    }

    pub fn card(children: Vec<Node>) -> Node {
        Node::Card {
            bg: None,
            border: None,
            radius: None,
            pad: None,
            shadow: None,
            children,
        }
    }

    pub fn rule() -> Node {
        Node::Rule {
            thickness: None,
            color: None,
        }
    }

    pub fn spacer(height: f32) -> Node {
        Node::Spacer { height }
    }
}

pub const MAX_DEPTH: usize = 64;
pub const MAX_LEAVES: usize = 20_000;
pub const MAX_COLUMNS: u8 = 32;

// Fail-fast boundary: every problem is reported at once, not just the first, so
// an agent can fix a spec in one round trip instead of play whack-a-mole.
pub fn validate(doc: &Document) -> Vec<String> {
    let mut errors = Vec::new();
    let frames = doc.frames_effective();
    if frames.is_empty() {
        errors.push("spec defines no frames: give it `frames` or a top-level `canvas`".into());
        return errors;
    }
    for (name, frame) in &frames {
        validate_frame(name, frame, &mut errors);
    }
    errors
}

fn validate_frame(name: &str, frame: &Frame, errors: &mut Vec<String>) {
    let fx = |e: &str| {
        if name == "design" {
            e.to_string()
        } else {
            format!("frame {name}: {e}")
        }
    };
    if !(320.0..=8192.0).contains(&frame.canvas.width) {
        errors.push(fx(&format!(
            "canvas.width must be between 320 and 8192, got {}",
            frame.canvas.width
        )));
    }
    if !crate::verify::is_color(&frame.canvas.background) {
        errors.push(fx(&format!(
            "canvas.background is not a valid colour: {:?}",
            frame.canvas.background
        )));
    }
    let mut leaves = 0usize;
    walk(&frame.root, 0, false, &mut leaves, errors, &fx);
    if leaves > MAX_LEAVES {
        errors.push(fx(&format!(
            "spec has {leaves} leaf nodes, the limit is {MAX_LEAVES}"
        )));
    }
}

fn walk(
    node: &Node,
    depth: usize,
    in_grid: bool,
    leaves: &mut usize,
    errors: &mut Vec<String>,
    fx: &dyn Fn(&str) -> String,
) {
    if depth > MAX_DEPTH {
        errors.push(fx(&format!("spec nesting exceeds {MAX_DEPTH} levels")));
        return;
    }
    match node {
        Node::Stack {
            gap, pad, children, ..
        } => {
            opt_nonneg(errors, fx, "gap", *gap, depth);
            opt_nonneg(errors, fx, "pad", *pad, depth);
            for c in children {
                walk(c, depth + 1, false, leaves, errors, fx);
            }
        }
        Node::Grid {
            columns,
            gap,
            pad,
            children,
            ..
        } => {
            if *columns < 1 || *columns > MAX_COLUMNS {
                errors.push(fx(&format!(
                    "depth {depth}: columns must be 1..={MAX_COLUMNS}, got {columns}"
                )));
            }
            opt_nonneg(errors, fx, "gap", *gap, depth);
            opt_nonneg(errors, fx, "pad", *pad, depth);
            for c in children {
                walk(c, depth + 1, true, leaves, errors, fx);
            }
        }
        Node::Text {
            text,
            color,
            max_measure,
            tracking,
            ..
        } => {
            *leaves += 1;
            if text.trim().is_empty() {
                errors.push(fx(&format!("depth {depth}: text node is empty")));
            }
            if let Some(c) = color {
                check_color(errors, fx, "text color", c, depth);
            }
            match *max_measure {
                Some(m) if !m.is_finite() || m <= 0.0 => errors.push(fx(&format!(
                    "depth {depth}: max_measure must be a finite value > 0, got {m}"
                ))),
                _ => {}
            }
            if let Some(t) = tracking
                && !t.is_finite()
            {
                errors.push(fx(&format!(
                    "depth {depth}: tracking must be finite, got {t}"
                )));
            }
        }
        Node::Rule { thickness, color } => {
            *leaves += 1;
            if let Some(t) = thickness
                && (!t.is_finite() || *t <= 0.0)
            {
                errors.push(fx(&format!(
                    "depth {depth}: thickness must be a finite value > 0, got {t}"
                )));
            }
            if let Some(c) = color {
                check_color(errors, fx, "rule color", c, depth);
            }
        }
        Node::Spacer { height } => {
            *leaves += 1;
            if !height.is_finite() || *height < 0.0 {
                errors.push(fx(&format!(
                    "depth {depth}: spacer height must be a finite value >= 0, got {height}"
                )));
            }
        }
        Node::Card {
            bg,
            border,
            radius,
            pad,
            children,
            ..
        } => {
            opt_nonneg(errors, fx, "pad", *pad, depth);
            opt_nonneg(errors, fx, "radius", *radius, depth);
            check_fill(errors, fx, "card bg", bg.as_ref(), depth);
            if let Some(c) = border {
                check_color(errors, fx, "card border", c, depth);
            }
            for c in children {
                walk(c, depth + 1, false, leaves, errors, fx);
            }
        }
        Node::Cell { span, child } => {
            if !in_grid {
                errors.push(fx(&format!(
                    "depth {depth}: a cell may only be a direct child of a grid"
                )));
            }
            if *span < 1 || *span > MAX_COLUMNS {
                errors.push(fx(&format!(
                    "depth {depth}: cell span must be 1..={MAX_COLUMNS}, got {span}"
                )));
            }
            walk(child, depth + 1, false, leaves, errors, fx);
        }
        Node::Image {
            src, width, height, ..
        } => {
            *leaves += 1;
            if src.trim().is_empty() {
                errors.push(fx(&format!("depth {depth}: image src is empty")));
            }
            if let Some(w) = width
                && (!w.is_finite() || *w <= 0.0)
            {
                errors.push(fx(&format!(
                    "depth {depth}: image width must be a finite value > 0, got {w}"
                )));
            }
            if !height.is_finite() || *height <= 0.0 {
                errors.push(fx(&format!(
                    "depth {depth}: image height must be a finite value > 0, got {height}"
                )));
            }
            if let Some(bad) = unsafe_src(src) {
                errors.push(fx(&format!("depth {depth}: image src rejected ({bad})")));
            } else if !src.starts_with("data:") {
                errors.push(fx(&format!(
                    "depth {depth}: image src {src:?} is not a data URI — it must be resolved against asset_dir at load time"
                )));
            }
        }
        Node::Raw { svg, height } => {
            *leaves += 1;
            if svg.trim().is_empty() {
                errors.push(fx(&format!("depth {depth}: raw svg is empty")));
            }
            if let Some(h) = height
                && (!h.is_finite() || *h < 0.0)
            {
                errors.push(fx(&format!(
                    "depth {depth}: raw height must be a finite value >= 0, got {h}"
                )));
            }
            if let Some(bad) = unsafe_svg(svg) {
                errors.push(fx(&format!("depth {depth}: raw svg rejected ({bad})")));
            }
        }
        Node::Component { name } => {
            *leaves += 1;
            errors.push(fx(&format!(
                "depth {depth}: component {name:?} was never expanded"
            )));
        }
    }
}

// `raw` is the one escape hatch that injects arbitrary SVG into the document,
// so it is also the one place an untrusted spec can smuggle script execution or
// an XXE entity read. This is a denylist on the constructs that matter; the
// real defence is that resvg never executes script at all, and we never resolve
// external entities.
fn unsafe_svg(svg: &str) -> Option<&'static str> {
    let lower = svg.to_ascii_lowercase();
    if lower.contains("<script") {
        return Some("<script is not allowed");
    }
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Some("doctype/entity declarations are not allowed");
    }
    if lower.contains("javascript:") {
        return Some("javascript: urls are not allowed");
    }
    if lower.contains("data:text/html") {
        return Some("data:text/html urls are not allowed");
    }
    None
}

fn unsafe_src(src: &str) -> Option<&'static str> {
    let lower = src.to_ascii_lowercase();
    if lower.contains("javascript:") {
        return Some("javascript: urls are not allowed");
    }
    if lower.contains("<") {
        return Some("markup is not allowed in src");
    }
    None
}

fn opt_nonneg(
    errors: &mut Vec<String>,
    fx: &dyn Fn(&str) -> String,
    name: &str,
    v: Option<f32>,
    depth: usize,
) {
    if let Some(v) = v
        && (!v.is_finite() || v < 0.0)
    {
        errors.push(fx(&format!(
            "depth {depth}: {name} must be a finite value >= 0, got {v}"
        )));
    }
}

fn check_color(
    errors: &mut Vec<String>,
    fx: &dyn Fn(&str) -> String,
    what: &str,
    value: &str,
    depth: usize,
) {
    if !crate::verify::is_color(value) {
        errors.push(fx(&format!(
            "depth {depth}: {what} is not a valid colour: {value:?}"
        )));
    }
}

fn check_fill(
    errors: &mut Vec<String>,
    fx: &dyn Fn(&str) -> String,
    what: &str,
    fill: Option<&Fill>,
    depth: usize,
) {
    match fill {
        None => {}
        Some(Fill::Solid(c)) if !crate::verify::is_color(c) => errors.push(fx(&format!(
            "depth {depth}: {what} is not a valid colour: {c:?}"
        ))),
        Some(Fill::Linear(g)) => {
            for (label, c) in [("from", &g.from), ("to", &g.to)] {
                if !crate::verify::is_color(c) {
                    errors.push(fx(&format!(
                        "depth {depth}: {what} gradient {label} is not a valid colour: {c:?}"
                    )));
                }
            }
            if !g.angle.is_finite() {
                errors.push(fx(&format!(
                    "depth {depth}: {what} gradient angle must be finite, got {}",
                    g.angle
                )));
            }
        }
        Some(Fill::Solid(_)) => {}
    }
}

/// Expansion of `component` references happens before validation: a component
/// body is inlined into the referencing tree so responsive frames can share a
/// header or product card instead of duplicating it. Cycles and unknown names
/// are reported, and the inlined children are expanded in turn.
pub fn expand_components(doc: &mut Document, errors: &mut Vec<String>) {
    let comps = doc.components.clone();
    if doc.frames.is_empty() {
        if let Some(root) = &mut doc.root {
            expand_node(root, &comps, &mut Vec::new(), errors);
        }
    } else {
        for frame in doc.frames.values_mut() {
            expand_node(&mut frame.root, &comps, &mut Vec::new(), errors);
        }
    }
}

fn expand_node(
    node: &mut Node,
    comps: &BTreeMap<String, Node>,
    active: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    match node {
        Node::Stack { children, .. }
        | Node::Grid { children, .. }
        | Node::Card { children, .. } => {
            for c in children.iter_mut() {
                expand_node(c, comps, active, errors);
            }
        }
        Node::Cell { child, .. } => expand_node(child, comps, active, errors),
        Node::Component { name } => {
            if active.iter().any(|a| a == name) {
                let mut chain = active.join(" -> ");
                chain.push_str(" -> ");
                chain.push_str(name);
                errors.push(format!("component cycle: {chain}"));
                return;
            }
            match comps.get(name) {
                None => errors.push(format!("unknown component {name:?}")),
                Some(body) => {
                    active.push(name.clone());
                    let mut replacement = body.clone();
                    expand_node(&mut replacement, comps, active, errors);
                    active.pop();
                    *node = replacement;
                }
            }
        }
        _ => {}
    }
}

/// Resolve relative `image` srcs against `base/asset_dir` into data URIs before
/// validation, so the emitted SVG is self-contained and layout stays
/// infallible. `data:` URIs pass through untouched.
pub fn resolve_images(doc: &mut Document, base: &Path, errors: &mut Vec<String>) {
    let asset = doc.asset_dir.clone().unwrap_or_else(|| ".".into());
    let mut resolve_root = |root: &mut Node, frame: &str| {
        walk_images(root, &mut |src| {
            if src.starts_with("data:") {
                return src.clone();
            }
            let p = base.join(&asset).join(src);
            match std::fs::read(&p) {
                Ok(bytes) => format!("data:{};base64,{}", mime_for(&p), b64_encode(&bytes)),
                Err(e) => {
                    errors.push(format!(
                        "frame {frame}: image src {src:?} is not resolvable at {}: {e}",
                        p.display()
                    ));
                    src.clone()
                }
            }
        });
    };
    if doc.frames.is_empty() {
        if let Some(root) = &mut doc.root {
            resolve_root(root, "design");
        }
    } else {
        for (name, frame) in doc.frames.iter_mut() {
            resolve_root(&mut frame.root, name);
        }
    }
}

fn walk_images(node: &mut Node, f: &mut dyn FnMut(&String) -> String) {
    match node {
        Node::Stack { children, .. }
        | Node::Grid { children, .. }
        | Node::Card { children, .. } => {
            for c in children.iter_mut() {
                walk_images(c, f);
            }
        }
        Node::Cell { child, .. } => walk_images(child, f),
        Node::Image { src, .. } => *src = f(src),
        _ => {}
    }
}

fn mime_for(p: &Path) -> &'static str {
    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) if ext == "png" => "image/png",
        Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg",
        Some(ext) if ext == "webp" => "image/webp",
        Some(ext) if ext == "gif" => "image/gif",
        Some(ext) if ext == "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn b64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
