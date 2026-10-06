use crate::primitive::{Prim, Scene};
use crate::theme::Theme;

#[derive(Debug, Default)]
pub struct Report {
    pub prims: usize,
    pub width: f32,
    pub height: f32,
    pub errors: Vec<String>,
    pub out_of_bounds: Vec<String>,
    pub collisions: Vec<String>,
    pub contrast: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.errors.is_empty() && self.out_of_bounds.is_empty() && self.collisions.is_empty()
    }

    pub fn passed(&self, strict: bool) -> bool {
        if strict {
            self.ok() && self.contrast.is_empty()
        } else {
            self.ok()
        }
    }

    pub fn summary(&self, strict: bool) -> String {
        let status = match (self.ok(), self.contrast.is_empty()) {
            (true, true) => "PASS",
            (true, false) => {
                if strict {
                    "FAIL"
                } else {
                    "PASS (warnings)"
                }
            }
            (false, _) => "FAIL",
        };
        format!(
            "prims={} size={}x{} errors={} out_of_bounds={} collisions={} contrast={} -- {}",
            self.prims,
            round(self.width),
            round(self.height),
            self.errors.len(),
            self.out_of_bounds.len(),
            self.collisions.len(),
            self.contrast.len(),
            status
        )
    }

    pub fn details(&self) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        for e in &self.errors {
            all.push(format!("error: {e}"));
        }
        for e in &self.out_of_bounds {
            all.push(format!("bounds: {e}"));
        }
        for e in &self.collisions {
            all.push(format!("collision: {e}"));
        }
        for e in &self.contrast {
            all.push(format!("contrast: {e}"));
        }
        all
    }
}

fn round(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

pub fn scene(scene: &Scene, pad: f32) -> Report {
    let mut rep = Report {
        prims: scene.prims.len(),
        width: scene.width,
        height: scene.height,
        ..Default::default()
    };

    check_extent(&mut rep, scene);

    for (i, p) in scene.prims.iter().enumerate() {
        let (x, y, w, h) = p.bounds();
        if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
            rep.errors.push(format!(
                "#{i} {} has non-finite geometry at ({x},{y}) {w}x{h}",
                p.kind()
            ));
            continue;
        }
        if x < -pad || y < -pad || x + w > scene.width + pad || y + h > scene.height + pad {
            rep.out_of_bounds.push(format!(
                "#{i} {} at ({},{}) {}x{} exceeds {}x{}",
                p.kind(),
                round(x),
                round(y),
                round(w),
                round(h),
                round(scene.width),
                round(scene.height)
            ));
        }
        if let Prim::Raw { body, .. } = p
            && let Err(e) = raw_parses(body)
        {
            rep.errors.push(format!("#{i} raw svg is invalid: {e}"));
        }
    }

    let mut rects: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
    for (i, p) in scene.prims.iter().enumerate() {
        if let Prim::Text { .. } = p {
            let (x, y, w, h) = p.bounds();
            rects.push((i, x, y, w, h));
        }
    }
    rects.sort_by(|a, b| a.2.total_cmp(&b.2));
    if rects.len() > MAX_COLLISION_CHECK {
        rep.errors.push(format!(
            "{} text primitives is above the {} limit for collision checking",
            rects.len(),
            MAX_COLLISION_CHECK
        ));
        rects.clear();
    }
    for a in 0..rects.len() {
        let (ia, ax, ay, aw, ah) = rects[a];
        for &(ib, bx, by, bw, bh) in &rects[a + 1..] {
            if by >= ay + ah {
                break;
            }
            if bx >= ax + aw || bx + bw <= ax {
                continue;
            }
            let ox = (ax + aw).min(bx + bw) - ax.max(bx);
            let oy = (ay + ah).min(by + bh) - ay.max(by);
            if ox > 1.5 && oy > 1.5 {
                rep.collisions.push(format!(
                    "#{ia} ({},{}) {}x{} x #{ib} ({},{}) {}x{} overlap {}x{}",
                    round(ax),
                    round(ay),
                    round(aw),
                    round(ah),
                    round(bx),
                    round(by),
                    round(bw),
                    round(bh),
                    round(ox),
                    round(oy)
                ));
            }
        }
    }

    rep
}

pub const MAX_DIM: f32 = 16000.0;
pub const MAX_COLLISION_CHECK: usize = 20_000;

fn check_extent(rep: &mut Report, scene: &Scene) {
    if !scene.width.is_finite() || !scene.height.is_finite() {
        rep.errors.push(format!(
            "canvas geometry is not finite: {}x{}",
            scene.width, scene.height
        ));
        return;
    }
    if scene.width < 1.0 || scene.height < 1.0 {
        rep.errors.push(format!(
            "canvas must be at least 1x1, got {}x{}",
            round(scene.width),
            round(scene.height)
        ));
        return;
    }
    if scene.width > MAX_DIM || scene.height > MAX_DIM {
        rep.errors.push(format!(
            "canvas {}x{} exceeds the {}px raster limit; reduce width or content",
            round(scene.width),
            round(scene.height),
            MAX_DIM as i64
        ));
    }
}

fn raw_parses(body: &str) -> std::result::Result<(), String> {
    if !body.trim_start().starts_with('<') {
        return Err("expected markup starting with '<'".into());
    }
    let doc = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\">{body}</svg>"
    );
    usvg::Tree::from_data(doc.as_bytes(), &usvg::Options::default())
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn theme(theme: &Theme) -> Vec<String> {
    let pairs = [
        ("ink", &theme.palette.ink, "bg", &theme.palette.bg, 4.5),
        ("muted", &theme.palette.muted, "bg", &theme.palette.bg, 4.5),
        (
            "accent",
            &theme.palette.accent,
            "bg",
            &theme.palette.bg,
            4.5,
        ),
        (
            "ink",
            &theme.palette.ink,
            "surface",
            &theme.palette.surface,
            4.5,
        ),
        (
            "muted",
            &theme.palette.muted,
            "surface",
            &theme.palette.surface,
            4.5,
        ),
        (
            "accent",
            &theme.palette.accent,
            "surface",
            &theme.palette.surface,
            4.5,
        ),
        ("line", &theme.palette.line, "bg", &theme.palette.bg, 1.5),
    ];
    let mut out = Vec::new();
    for (fa, a, fb, b, min) in pairs {
        match (parse_hex(a), parse_hex(b)) {
            (Some(c1), Some(c2)) => {
                let r = contrast(c1, c2);
                if r < min {
                    out.push(format!(
                        "{fa}/{fb} ({a} on {b}) ratio {r:.2} < required {min}"
                    ));
                }
            }
            _ => out.push(format!("unparseable colour in {fa} or {fb}")),
        }
    }
    out
}

pub fn parse_hex(s: &str) -> Option<(f64, f64, f64)> {
    let h = s.trim().trim_start_matches('#');
    let digits: Vec<char> = match h.chars().count() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.chars().collect(),
        _ => return None,
    };
    if !digits.iter().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let expanded: String = digits.into_iter().collect();
    let v = u32::from_str_radix(&expanded, 16).ok()?;
    Some((
        ((v >> 16) & 0xff) as f64 / 255.0,
        ((v >> 8) & 0xff) as f64 / 255.0,
        (v & 0xff) as f64 / 255.0,
    ))
}

pub fn is_color(s: &str) -> bool {
    let t = s.trim();
    parse_hex(t).is_some() || matches!(t, "transparent" | "none" | "currentColor")
}

fn lum(c: (f64, f64, f64)) -> f64 {
    let f = |v: f64| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

pub fn contrast(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let (l1, l2) = (lum(a), lum(b));
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}
