use crate::fonts::FontBook;
use crate::primitive::{Prim, Scene};
use crate::spec::{Align, Node};
use crate::theme::{TextRole, Theme};

pub struct Ctx<'a> {
    pub theme: &'a Theme,
    pub fonts: &'a FontBook,
}

#[derive(Debug, Clone, Copy)]
pub struct Extent {
    pub w: f32,
    pub h: f32,
}

fn weight_for(role: TextRole) -> u16 {
    match role {
        TextRole::Body | TextRole::Small => 400,
        _ => 700,
    }
}

fn measure_text(
    node_text: &str,
    role: TextRole,
    max_measure: Option<f32>,
    avail: f32,
    ctx: &Ctx,
) -> (Vec<String>, f32, f32) {
    let size = ctx.theme.size(role);
    let family = ctx.theme.family_for(role);
    let weight = weight_for(role);
    let cap = max_measure.unwrap_or(ctx.theme.type_scale.measure);
    let measure = cap.min(avail).max(size * 4.0);
    let lines = ctx.fonts.wrap(family, weight, size, measure, node_text);
    let leading = ctx.theme.leading(role);
    let h = lines.len() as f32 * leading;
    let widest = lines
        .iter()
        .map(|l| ctx.fonts.advance(family, weight, size, l))
        .fold(0.0f32, f32::max);
    (lines, widest, h)
}

pub fn measure(node: &Node, avail: f32, ctx: &Ctx) -> Extent {
    match node {
        Node::Stack {
            gap, pad, children, ..
        } => {
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(ctx.theme.space.base);
            let inner = (avail - pad * 2.0).max(1.0);
            let mut h = 0.0f32;
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    h += gap;
                }
                h += measure(child, inner, ctx).h;
            }
            Extent {
                w: avail,
                h: h + pad * 2.0,
            }
        }
        Node::Grid {
            columns,
            gap,
            pad,
            children,
            ..
        } => {
            let cols = (*columns).max(1) as usize;
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(ctx.theme.space.base);
            let inner = ((avail - pad * 2.0 - gap * (cols as f32 - 1.0)).max(1.0)) / cols as f32;
            let mut h = 0.0f32;
            for row in children.chunks(cols) {
                let rh = row
                    .iter()
                    .map(|c| measure(c, inner, ctx).h)
                    .fold(0.0f32, f32::max);
                h += rh;
            }
            let rows = children.len().div_ceil(cols).max(1) as f32;
            Extent {
                w: avail,
                h: h + gap * (rows - 1.0) + pad * 2.0,
            }
        }
        Node::Text {
            text,
            role,
            max_measure,
            ..
        } => {
            let (_, _, h) = measure_text(text, *role, *max_measure, avail, ctx);
            Extent { w: avail, h }
        }
        Node::Rule { thickness, .. } => Extent {
            w: avail,
            h: thickness.unwrap_or(1.0),
        },
        Node::Spacer { height } => Extent {
            w: avail,
            h: *height,
        },
        Node::Card { pad, children, .. } => {
            let pad = pad.unwrap_or(ctx.theme.space.base * 3.0);
            let inner = (avail - pad * 2.0).max(1.0);
            let mut h = 0.0f32;
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    h += ctx.theme.space.base;
                }
                h += measure(child, inner, ctx).h;
            }
            Extent {
                w: avail,
                h: h + pad * 2.0,
            }
        }
        Node::Raw { height, .. } => Extent {
            w: avail,
            h: height.unwrap_or(0.0),
        },
    }
}

pub fn build(doc: &crate::spec::Document, ctx: &Ctx) -> Scene {
    let total = measure(&doc.root, doc.canvas.width, ctx);
    let mut scene = Scene {
        width: doc.canvas.width,
        height: total.h.ceil(),
        background: doc.canvas.background.clone(),
        prims: Vec::new(),
    };
    scene.prims.push(Prim::Rect {
        x: 0.0,
        y: 0.0,
        w: doc.canvas.width,
        h: scene.height,
        fill: doc.canvas.background.clone(),
        radius: 0.0,
        stroke: None,
        stroke_width: 0.0,
    });
    place(&doc.root, 0.0, 0.0, doc.canvas.width, ctx, &mut scene.prims);
    scene
}

fn place(node: &Node, x: f32, y: f32, avail: f32, ctx: &Ctx, out: &mut Vec<Prim>) {
    let theme = ctx.theme;
    match node {
        Node::Stack {
            gap,
            align,
            pad,
            bg,
            children,
        } => {
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(theme.space.base);
            let align = align.unwrap_or(Align::Start);
            let inner_w = (avail - pad * 2.0).max(1.0);
            if let Some(fill) = bg {
                let h = measure(node, avail, ctx).h;
                out.push(Prim::Rect {
                    x,
                    y,
                    w: avail,
                    h,
                    fill: fill.clone(),
                    radius: 0.0,
                    stroke: None,
                    stroke_width: 0.0,
                });
            }
            let mut cy = y + pad;
            for child in children {
                if cy > y + pad {
                    cy += gap;
                }
                let ext = measure(child, inner_w, ctx);
                let (cx, cw) = match align {
                    Align::Start => (x + pad, inner_w),
                    Align::Center => (x + pad + (inner_w - ext.w) / 2.0, ext.w),
                    Align::End => (x + avail - pad - ext.w, ext.w),
                };
                place(child, cx, cy, cw, ctx, out);
                cy += ext.h;
            }
        }
        Node::Grid {
            columns,
            gap,
            align,
            pad,
            bg,
            children,
        } => {
            let cols = (*columns).max(1) as usize;
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(theme.space.base);
            let align = align.unwrap_or(Align::Start);
            let inner_w = (avail - pad * 2.0 - gap * (cols - 1) as f32).max(1.0);
            let col_w = inner_w / cols as f32;
            if let Some(fill) = bg {
                let h = measure(node, avail, ctx).h;
                out.push(Prim::Rect {
                    x,
                    y,
                    w: avail,
                    h,
                    fill: fill.clone(),
                    radius: 0.0,
                    stroke: None,
                    stroke_width: 0.0,
                });
            }
            let mut cy = y + pad;
            for row in children.chunks(cols) {
                let mut rh = 0.0f32;
                for child in row {
                    rh = rh.max(measure(child, col_w, ctx).h);
                }
                for (i, child) in row.iter().enumerate() {
                    let ext = measure(child, col_w, ctx);
                    let col_x = x + pad + i as f32 * (col_w + gap);
                    let cx = match align {
                        Align::Start | Align::Center => col_x,
                        Align::End => col_x + (col_w - ext.w),
                    };
                    place(child, cx, cy, col_w, ctx, out);
                }
                cy += rh;
            }
        }
        Node::Text {
            text,
            role,
            text_align,
            color,
            max_measure,
        } => {
            let align = text_align.unwrap_or(Align::Start);
            let (lines, widest, _) = measure_text(text, *role, *max_measure, avail, ctx);
            let size = theme.size(*role);
            let family = theme.family_for(*role).to_string();
            let weight = weight_for(*role);
            let leading = theme.leading(*role);
            let fill = color
                .clone()
                .unwrap_or_else(|| theme.ink_for(*role).to_string());
            let baseline_pad = size * 0.82;
            for (i, line) in lines.iter().enumerate() {
                let ty = y + i as f32 * leading;
                let lx = match align {
                    Align::Start => x,
                    Align::Center => x + (avail - widest) / 2.0,
                    Align::End => x + avail - widest,
                };
                out.push(Prim::Text {
                    x: lx,
                    y: ty + baseline_pad,
                    w: widest,
                    h: leading,
                    text: line.clone(),
                    size,
                    family: family.clone(),
                    weight,
                    fill: fill.clone(),
                    align,
                });
            }
        }
        Node::Rule { thickness, color } => {
            let t = thickness.unwrap_or(1.0);
            out.push(Prim::Rule {
                x,
                y: y.round(),
                w: avail,
                thickness: t,
                stroke: color.clone().unwrap_or_else(|| theme.palette.line.clone()),
            });
        }
        Node::Spacer { .. } => {}
        Node::Card {
            bg,
            border,
            radius,
            pad,
            children,
        } => {
            let pad = pad.unwrap_or(theme.space.base * 3.0);
            let ext = measure(node, avail, ctx);
            out.push(Prim::Rect {
                x,
                y,
                w: avail,
                h: ext.h,
                fill: bg.clone().unwrap_or_else(|| theme.palette.surface.clone()),
                radius: radius.unwrap_or(theme.radius.md),
                stroke: border.clone(),
                stroke_width: if border.is_some() { 1.0 } else { 0.0 },
            });
            let mut cy = y + pad;
            for child in children {
                if cy > y + pad {
                    cy += theme.space.base;
                }
                let inner = (avail - pad * 2.0).max(1.0);
                let child_ext = measure(child, inner, ctx);
                place(child, x + pad, cy, inner, ctx, out);
                cy += child_ext.h;
            }
        }
        Node::Raw { svg, .. } => {
            out.push(Prim::Raw {
                x,
                y,
                body: svg.clone(),
            });
        }
    }
}
