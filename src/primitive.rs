use crate::spec::Align;

#[derive(Debug, Clone)]
pub struct Scene {
    pub width: f32,
    pub height: f32,
    pub background: String,
    pub prims: Vec<Prim>,
}

#[derive(Debug, Clone)]
pub enum Prim {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: String,
        radius: f32,
        stroke: Option<String>,
        stroke_width: f32,
    },
    Text {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: String,
        size: f32,
        family: String,
        weight: u16,
        fill: String,
        align: Align,
    },
    Rule {
        x: f32,
        y: f32,
        w: f32,
        thickness: f32,
        stroke: String,
    },
    Raw {
        x: f32,
        y: f32,
        body: String,
    },
}

impl Prim {
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        match self {
            Prim::Rect { x, y, w, h, .. } => (*x, *y, *w, *h),
            Prim::Rule {
                x, y, w, thickness, ..
            } => (*x, *y, *w, *thickness),
            Prim::Text {
                x, y, w, h, size, ..
            } => (*x, *y - *size * 0.82, *w, *h),
            Prim::Raw { x, y, .. } => (*x, *y, 0.0, 0.0),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Prim::Rect { .. } => "rect",
            Prim::Text { .. } => "text",
            Prim::Rule { .. } => "rule",
            Prim::Raw { .. } => "raw",
        }
    }
}
