use std::fmt::Write as _;

use crate::primitive::{Prim, Scene};

pub fn family_emit(name: &str) -> &str {
    match name {
        "serif" => "DejaVu Serif",
        _ => "Noto Sans",
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn n(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

pub fn render(scene: &Scene) -> String {
    let w = n(scene.width);
    let h = n(scene.height);
    let bg = esc(&scene.background);
    let mut s = String::with_capacity(scene.prims.len() * 180 + 256);
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    let _ = write!(
        s,
        "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>"
    );

    for p in &scene.prims {
        match p {
            Prim::Rect {
                x,
                y,
                w,
                h,
                fill,
                radius,
                stroke,
                stroke_width,
            } => {
                let _ = write!(
                    s,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"",
                    n(*x),
                    n(*y),
                    n(*w),
                    n(*h),
                    esc(fill)
                );
                if *radius > 0.0 {
                    let _ = write!(s, " rx=\"{radius}\" ry=\"{radius}\"");
                }
                if let Some(st) = stroke {
                    let _ = write!(
                        s,
                        " stroke=\"{}\" stroke-width=\"{}\"",
                        esc(st),
                        n(*stroke_width)
                    );
                }
                s.push_str("/>");
            }
            Prim::Text {
                x,
                y,
                size,
                text,
                family,
                weight,
                fill,
                ..
            } => {
                let _ = write!(
                    s,
                    "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"{}\" xml:space=\"preserve\">{}</text>",
                    n(*x),
                    n(*y),
                    esc(family_emit(family)),
                    n(*size),
                    weight,
                    esc(fill),
                    esc(text)
                );
            }
            Prim::Rule {
                x,
                y,
                w,
                thickness,
                stroke,
            } => {
                let _ = write!(
                    s,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    n(*x),
                    n(*y),
                    n(*w),
                    n(*thickness),
                    esc(stroke)
                );
            }
            Prim::Raw { x, y, body } => {
                let _ = write!(
                    s,
                    "<g transform=\"translate({},{})\">{}</g>",
                    n(*x),
                    n(*y),
                    body
                );
            }
        }
    }
    s.push_str("</svg>");
    s
}
