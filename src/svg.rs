use std::collections::HashMap;
use std::fmt::Write as _;

use crate::primitive::{Prim, Scene};
use crate::spec::Fit;

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
    let mut s = String::with_capacity(scene.prims.len() * 200 + 256);
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    let _ = write!(
        s,
        "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>"
    );

    // Deterministic defs: unique gradient (from/to/angle) and shadow slots,
    // collected in first-seen order so output is reproducible.
    let mut gradients: HashMap<(String, String, i32), usize> = HashMap::new();
    let mut shadows: HashMap<(i32, i32, i32, String), usize> = HashMap::new();
    for p in &scene.prims {
        if let Prim::Rect {
            gradient, shadow, ..
        } = p
        {
            if let Some((from, to, angle)) = gradient {
                let key = (from.clone(), to.clone(), (angle * 10.0).round() as i32);
                if !gradients.contains_key(&key) {
                    let id = gradients.len();
                    gradients.insert(key, id);
                }
            }
            if let Some(sh) = shadow {
                let key = (
                    (sh.blur * 20.0).round() as i32,
                    (sh.x * 20.0).round() as i32,
                    (sh.y * 20.0).round() as i32,
                    sh.color.clone(),
                );
                if !shadows.contains_key(&key) {
                    let id = shadows.len();
                    shadows.insert(key, id);
                }
            }
        }
    }
    if !gradients.is_empty() || !shadows.is_empty() {
        s.push_str("<defs>");
        for ((from, to, angle), id) in &gradients {
            // Angle is the gradient direction in degrees (90 = top to bottom).
            let rad = (angle * 10) as f64 / 10.0 * std::f64::consts::PI / 180.0;
            let (dx, dy) = (rad.cos() as f32, rad.sin() as f32);
            let (x1, y1) = (0.5 - dx / 2.0, 0.5 - dy / 2.0);
            let (x2, y2) = (0.5 + dx / 2.0, 0.5 + dy / 2.0);
            let _ = write!(
                s,
                "<linearGradient id=\"g{id}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" gradientUnits=\"objectBoundingBox\">"
            );
            let _ = write!(s, "<stop offset=\"0\" stop-color=\"{}\"/>", esc(from));
            let _ = write!(s, "<stop offset=\"1\" stop-color=\"{}\"/>", esc(to));
            s.push_str("</linearGradient>");
        }
        for ((blur, x, y, color), id) in &shadows {
            let _ = write!(
                s,
                "<filter id=\"f{id}\" x=\"-60%\" y=\"-60%\" width=\"220%\" height=\"220%\">"
            );
            let _ = write!(
                s,
                "<feDropShadow dx=\"{}\" dy=\"{}\" stdDeviation=\"{}\" flood-color=\"{}\"/>",
                n(*x as f32 / 20.0),
                n(*y as f32 / 20.0),
                n(*blur as f32 / 20.0),
                esc(color)
            );
            s.push_str("</filter>");
        }
        s.push_str("</defs>");
    }

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
                gradient,
                shadow,
            } => {
                let fill = match gradient {
                    Some((from, to, angle)) => {
                        let key = (from.clone(), to.clone(), (angle * 10.0).round() as i32);
                        format!("url(#g{})", gradients[&key])
                    }
                    None => esc(fill),
                };
                let _ = write!(
                    s,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\"",
                    n(*x),
                    n(*y),
                    n(*w),
                    n(*h)
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
                if let Some(sh) = shadow {
                    let key = (
                        (sh.blur * 20.0).round() as i32,
                        (sh.x * 20.0).round() as i32,
                        (sh.y * 20.0).round() as i32,
                        sh.color.clone(),
                    );
                    let _ = write!(s, " filter=\"url(#f{})\"", shadows[&key]);
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
                tracking,
                ..
            } => {
                let _ = write!(
                    s,
                    "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"{}\"",
                    n(*x),
                    n(*y),
                    esc(family_emit(family)),
                    n(*size),
                    weight,
                    esc(fill)
                );
                if *tracking != 0.0 {
                    let _ = write!(s, " letter-spacing=\"{}\"", n(*tracking));
                }
                let _ = write!(s, " xml:space=\"preserve\">{}</text>", esc(text));
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
            Prim::Image {
                x,
                y,
                w,
                h,
                href,
                fit,
            } => {
                let par = match fit {
                    Fit::Cover => "xMidYMid slice",
                    Fit::Contain => "xMidYMid meet",
                    Fit::Fill => "none",
                };
                let _ = write!(
                    s,
                    "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"{par}\" href=\"{}\"/>",
                    n(*x),
                    n(*y),
                    n(*w),
                    n(*h),
                    esc(href)
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
