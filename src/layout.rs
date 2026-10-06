use crate::fonts::FontBook;
use crate::primitive::{Backdrop, Prim, Scene};
use crate::spec::{Align, Fill, Frame, Node, Shadow};
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
    tracking: f32,
    ctx: &Ctx,
) -> (Vec<String>, f32, f32) {
    let size = ctx.theme.size(role);
    let family = ctx.theme.family_for(role);
    let weight = weight_for(role);
    let cap = max_measure.unwrap_or(ctx.theme.type_scale.measure);
    let measure = cap.min(avail).max(size * 4.0);
    let tracking_px = tracking * size;
    let lines = ctx
        .fonts
        .wrap(family, weight, size, measure, node_text, tracking_px);
    let leading = ctx.theme.leading(role);
    let h = lines.len() as f32 * leading;
    let widest = lines
        .iter()
        .map(|l| {
            let base = ctx.fonts.advance(family, weight, size, l);
            let gaps = l.chars().count().saturating_sub(1) as f32 * tracking_px;
            base + gaps
        })
        .fold(0.0f32, f32::max);
    (lines, widest, h)
}

fn span_of(node: &Node) -> (u8, &Node) {
    match node {
        Node::Cell { span, child } => (*span, child.as_ref()),
        other => (1, other),
    }
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
            let (_, content_h) = grid_metrics(children, cols, inner, gap, ctx);
            Extent {
                w: avail,
                h: content_h + pad * 2.0,
            }
        }
        Node::Text {
            text,
            role,
            max_measure,
            tracking,
            ..
        } => {
            let (_, _, h) = measure_text(
                text,
                *role,
                *max_measure,
                avail,
                tracking.unwrap_or(0.0),
                ctx,
            );
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
        Node::Image { width, height, .. } => Extent {
            w: width.unwrap_or(avail),
            h: *height,
        },
        Node::Raw { height, .. } => Extent {
            w: avail,
            h: height.unwrap_or(0.0),
        },
        Node::Cell { child, .. } => measure(child, avail, ctx),
        Node::Component { .. } => Extent { w: avail, h: 0.0 },
    }
}

/// Lays out a grid's children with column spans. Returns one rect per child
/// (aligned to the child index; x/y relative to the grid's content box) and the
/// content height. Rows bind: a cell that does not fit in the remaining
/// columns wraps to the next row, then later cells fill the leftover slots —
/// the float-grid pack that makes asymmetric marketplaces possible.
fn grid_metrics(
    children: &[Node],
    cols: usize,
    col_w: f32,
    gap: f32,
    ctx: &Ctx,
) -> (Vec<(f32, f32, f32, f32)>, f32) {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut used = 0usize;
    for (i, child) in children.iter().enumerate() {
        let (span, _) = span_of(child);
        let span = (span as usize).min(cols);
        if used + span > cols {
            rows.push(std::mem::take(&mut cur));
            used = 0;
        }
        cur.push(i);
        used += span;
    }
    if !cur.is_empty() {
        rows.push(cur);
    }

    let mut rects = vec![(0.0f32, 0.0f32, 0.0f32, 0.0f32); children.len()];
    let mut y = 0.0f32;
    let mut content_h = 0.0f32;
    for row in &rows {
        let mut rh = 0.0f32;
        let mut col = 0usize;
        for &i in row {
            let (span, child_node) = span_of(&children[i]);
            let span = span as usize;
            let w = span as f32 * col_w + (span as f32 - 1.0) * gap;
            let h = measure(child_node, w, ctx).h;
            rects[i] = (col as f32 * (col_w + gap), y, w, h);
            rh = rh.max(h);
            col += span;
        }
        content_h = y + rh;
        y += rh + gap;
    }
    (rects, content_h)
}

/// Resolve a container fill into (solid fallback, optional gradient).
fn paint(fill: &Option<Fill>, default: &str) -> (String, Option<(String, String, f32)>) {
    match fill {
        None => (default.to_string(), None),
        Some(Fill::Solid(c)) => (c.clone(), None),
        Some(Fill::Linear(g)) => (
            g.from.clone(),
            Some((g.from.clone(), g.to.clone(), g.angle)),
        ),
    }
}

pub fn build(frame: &Frame, ctx: &Ctx) -> Scene {
    let total = measure(&frame.root, frame.canvas.width, ctx);
    let mut scene = Scene {
        width: frame.canvas.width,
        height: total.h.ceil(),
        background: frame.canvas.background.clone(),
        prims: Vec::new(),
    };
    push_rect(
        &mut scene.prims,
        0.0,
        0.0,
        frame.canvas.width,
        scene.height,
        RectStyle {
            bg: &Some(Fill::Solid(frame.canvas.background.clone())),
            shadow: &None,
            radius: 0.0,
            stroke: None,
            stroke_width: 0.0,
        },
        ctx.theme,
    );
    place(
        &frame.root,
        0.0,
        0.0,
        frame.canvas.width,
        ctx,
        &Backdrop::Solid(frame.canvas.background.clone()),
        &mut scene.prims,
    );
    scene
}

fn place(
    node: &Node,
    x: f32,
    y: f32,
    avail: f32,
    ctx: &Ctx,
    backdrop: &Backdrop,
    out: &mut Vec<Prim>,
) {
    let theme = ctx.theme;
    match node {
        Node::Stack {
            gap,
            align,
            pad,
            bg,
            shadow,
            children,
        } => {
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(theme.space.base);
            let align = align.unwrap_or(Align::Start);
            let inner_w = (avail - pad * 2.0).max(1.0);
            let child_backdrop = match bg {
                Some(fill) => Backdrop::from_fill(fill),
                None => backdrop.clone(),
            };
            if bg.is_some() {
                let h = measure(node, avail, ctx).h;
                push_rect(
                    out,
                    x,
                    y,
                    avail,
                    h,
                    RectStyle {
                        bg,
                        shadow,
                        radius: 0.0,
                        stroke: None,
                        stroke_width: 0.0,
                    },
                    theme,
                );
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
                place(child, cx, cy, cw, ctx, &child_backdrop, out);
                cy += ext.h;
            }
        }
        Node::Grid {
            columns,
            gap,
            align,
            pad,
            bg,
            shadow,
            children,
        } => {
            let cols = (*columns).max(1) as usize;
            let pad = pad.unwrap_or(0.0);
            let gap = gap.unwrap_or(theme.space.base);
            let _align = align.unwrap_or(Align::Start);
            let inner_w = (avail - pad * 2.0 - gap * (cols as f32 - 1.0)).max(1.0);
            let col_w = inner_w / cols as f32;
            if bg.is_some() {
                let h = measure(node, avail, ctx).h;
                push_rect(
                    out,
                    x,
                    y,
                    avail,
                    h,
                    RectStyle {
                        bg,
                        shadow,
                        radius: 0.0,
                        stroke: None,
                        stroke_width: 0.0,
                    },
                    theme,
                );
            }
            let (rects, _) = grid_metrics(children, cols, col_w, gap, ctx);
            let origin_x = x + pad;
            let origin_y = y + pad;
            let child_backdrop = match bg {
                Some(fill) => Backdrop::from_fill(fill),
                None => backdrop.clone(),
            };
            for (i, child) in children.iter().enumerate() {
                let (_, child_node) = span_of(child);
                let (gx, gy, w, _) = rects[i];
                place(
                    child_node,
                    origin_x + gx,
                    origin_y + gy,
                    w,
                    ctx,
                    &child_backdrop,
                    out,
                );
            }
        }
        Node::Text {
            text,
            role,
            text_align,
            color,
            max_measure,
            tracking,
        } => {
            let align = text_align.unwrap_or(Align::Start);
            let tracking = tracking.unwrap_or(0.0);
            let (lines, widest, _) = measure_text(text, *role, *max_measure, avail, tracking, ctx);
            let size = theme.size(*role);
            let family = theme.family_for(*role).to_string();
            let weight = weight_for(*role);
            let leading = theme.leading(*role);
            let tracking_px = tracking * size;
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
                    tracking: tracking_px,
                    backdrop: backdrop.clone(),
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
            shadow,
            children,
        } => {
            let pad = pad.unwrap_or(theme.space.base * 3.0);
            let ext = measure(node, avail, ctx);
            push_rect(
                out,
                x,
                y,
                avail,
                ext.h,
                RectStyle {
                    bg,
                    shadow,
                    radius: radius.unwrap_or(theme.radius.md),
                    stroke: border.clone(),
                    stroke_width: if border.is_some() { 1.0 } else { 0.0 },
                },
                theme,
            );
            let mut cy = y + pad;
            let card_backdrop = match bg {
                Some(fill) => Backdrop::from_fill(fill),
                None => Backdrop::Solid(theme.palette.surface.clone()),
            };
            for child in children {
                if cy > y + pad {
                    cy += theme.space.base;
                }
                let inner = (avail - pad * 2.0).max(1.0);
                let child_ext = measure(child, inner, ctx);
                place(child, x + pad, cy, inner, ctx, &card_backdrop, out);
                cy += child_ext.h;
            }
        }
        Node::Image {
            src,
            width,
            height,
            fit,
        } => {
            out.push(Prim::Image {
                x,
                y,
                w: width.unwrap_or(avail),
                h: *height,
                href: src.clone(),
                fit: *fit,
            });
        }
        Node::Raw { svg, .. } => {
            out.push(Prim::Raw {
                x,
                y,
                body: svg.clone(),
            });
        }
        Node::Cell { child, .. } => place(child, x, y, avail, ctx, backdrop, out),
        Node::Component { .. } => {}
    }
}

/// Push a container rect honouring fill (solid or gradient) and shadow.
struct RectStyle<'a> {
    bg: &'a Option<Fill>,
    shadow: &'a Option<Shadow>,
    radius: f32,
    stroke: Option<String>,
    stroke_width: f32,
}

fn push_rect(out: &mut Vec<Prim>, x: f32, y: f32, w: f32, h: f32, style: RectStyle, theme: &Theme) {
    let RectStyle {
        bg,
        shadow,
        radius,
        stroke,
        stroke_width,
    } = style;
    let default = theme.palette.surface.clone();
    let (fill, gradient) = paint(bg, &default);
    out.push(Prim::Rect {
        x,
        y,
        w,
        h,
        fill,
        radius,
        stroke,
        stroke_width,
        gradient,
        shadow: shadow.clone(),
    });
}
