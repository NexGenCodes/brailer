use serde::{Deserialize, Serialize};

use crate::theme::TextRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub canvas: Canvas,
    pub theme: String,
    #[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Node {
    Stack {
        gap: Option<f32>,
        align: Option<Align>,
        pad: Option<f32>,
        bg: Option<String>,
        children: Vec<Node>,
    },
    Grid {
        columns: u8,
        gap: Option<f32>,
        align: Option<Align>,
        pad: Option<f32>,
        bg: Option<String>,
        children: Vec<Node>,
    },
    Text {
        text: String,
        role: TextRole,
        #[serde(rename = "align")]
        text_align: Option<Align>,
        color: Option<String>,
        max_measure: Option<f32>,
    },
    Rule {
        thickness: Option<f32>,
        color: Option<String>,
    },
    Spacer {
        height: f32,
    },
    Card {
        bg: Option<String>,
        border: Option<String>,
        radius: Option<f32>,
        pad: Option<f32>,
        children: Vec<Node>,
    },
    Raw {
        svg: String,
        height: Option<f32>,
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
        }
    }

    pub fn card(children: Vec<Node>) -> Node {
        Node::Card {
            bg: None,
            border: None,
            radius: None,
            pad: None,
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

pub fn validate(doc: &Document) -> Vec<String> {
    let mut errors = Vec::new();
    if !(320.0..=8192.0).contains(&doc.canvas.width) {
        errors.push(format!(
            "canvas.width must be between 320 and 8192, got {}",
            doc.canvas.width
        ));
    }
    if !crate::verify::is_color(&doc.canvas.background) {
        errors.push(format!(
            "canvas.background is not a valid colour: {:?}",
            doc.canvas.background
        ));
    }
    let mut leaves = 0usize;
    walk(&doc.root, 0, &mut leaves, &mut errors);
    if leaves > MAX_LEAVES {
        errors.push(format!(
            "spec has {leaves} leaf nodes, the limit is {MAX_LEAVES}"
        ));
    }
    errors
}

fn walk(node: &Node, depth: usize, leaves: &mut usize, errors: &mut Vec<String>) {
    if depth > MAX_DEPTH {
        errors.push(format!("spec nesting exceeds {MAX_DEPTH} levels"));
        return;
    }
    match node {
        Node::Stack {
            gap, pad, children, ..
        } => {
            opt_nonneg(errors, "gap", *gap, depth);
            opt_nonneg(errors, "pad", *pad, depth);
            for c in children {
                walk(c, depth + 1, leaves, errors);
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
                errors.push(format!(
                    "depth {depth}: columns must be 1..={MAX_COLUMNS}, got {columns}"
                ));
            }
            opt_nonneg(errors, "gap", *gap, depth);
            opt_nonneg(errors, "pad", *pad, depth);
            for c in children {
                walk(c, depth + 1, leaves, errors);
            }
        }
        Node::Text {
            text,
            color,
            max_measure,
            ..
        } => {
            *leaves += 1;
            if text.trim().is_empty() {
                errors.push(format!("depth {depth}: text node is empty"));
            }
            if let Some(c) = color {
                check_color(errors, "text color", c, depth);
            }
            match *max_measure {
                Some(m) if !m.is_finite() || m <= 0.0 => errors.push(format!(
                    "depth {depth}: max_measure must be a finite value > 0, got {m}"
                )),
                _ => {}
            }
        }
        Node::Rule { thickness, color } => {
            *leaves += 1;
            if let Some(t) = thickness
                && (!t.is_finite() || *t <= 0.0)
            {
                errors.push(format!(
                    "depth {depth}: thickness must be a finite value > 0, got {t}"
                ));
            }
            if let Some(c) = color {
                check_color(errors, "rule color", c, depth);
            }
        }
        Node::Spacer { height } => {
            *leaves += 1;
            if !height.is_finite() || *height < 0.0 {
                errors.push(format!(
                    "depth {depth}: spacer height must be a finite value >= 0, got {height}"
                ));
            }
        }
        Node::Card {
            bg,
            border,
            radius,
            pad,
            children,
        } => {
            opt_nonneg(errors, "pad", *pad, depth);
            opt_nonneg(errors, "radius", *radius, depth);
            if let Some(c) = bg {
                check_color(errors, "card bg", c, depth);
            }
            if let Some(c) = border {
                check_color(errors, "card border", c, depth);
            }
            for c in children {
                walk(c, depth + 1, leaves, errors);
            }
        }
        Node::Raw { svg, height } => {
            *leaves += 1;
            if svg.trim().is_empty() {
                errors.push(format!("depth {depth}: raw svg is empty"));
            }
            if let Some(h) = height
                && (!h.is_finite() || *h < 0.0)
            {
                errors.push(format!(
                    "depth {depth}: raw height must be a finite value >= 0, got {h}"
                ));
            }
            if let Some(bad) = unsafe_svg(svg) {
                errors.push(format!("depth {depth}: raw svg rejected ({bad})"));
            }
        }
    }
}

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

fn opt_nonneg(errors: &mut Vec<String>, name: &str, v: Option<f32>, depth: usize) {
    if let Some(v) = v
        && (!v.is_finite() || v < 0.0)
    {
        errors.push(format!(
            "depth {depth}: {name} must be a finite value >= 0, got {v}"
        ));
    }
}

fn check_color(errors: &mut Vec<String>, what: &str, value: &str, depth: usize) {
    if !crate::verify::is_color(value) {
        errors.push(format!(
            "depth {depth}: {what} is not a valid colour: {value:?}"
        ));
    }
}
