use std::collections::HashSet;
use std::io::Cursor;

use brailer::primitive::Prim;
use brailer::render;

fn example() -> (brailer::Pipeline, brailer::Scene) {
    // Self-contained legacy fixture (canvas+root -> the implicit `design`
    // frame) that exercises every palette token: ink headings, muted body,
    // accent label, line rule, surface card on a paper canvas.
    let json = r##"{"canvas":{"width":1440,"background":"#FAF7F2"},"theme":"editorial","root":{"kind":"stack","gap":16,"pad":48,"children":[
        {"kind":"text","text":"NEW WORK","role":"label"},
        {"kind":"text","text":"Solid ash, hand-planed","role":"h1"},
        {"kind":"text","text":"A seat three, without a central leg, finished in one workshop.","role":"body"},
        {"kind":"rule"},
        {"kind":"card","children":[
            {"kind":"text","text":"Oak","role":"h3"},
            {"kind":"text","text":"Quarter-sawn, air-dried.","role":"small"}
        ]}
    ]}}"##;
    let doc: brailer::spec::Document = serde_json::from_str(json).expect("parse");
    assert!(brailer::spec::validate(&doc).is_empty());
    let pipe = brailer::Pipeline::new(&doc.theme).expect("theme");
    let frame = doc
        .frames_effective()
        .remove("design")
        .expect("legacy spec maps to a design frame");
    let scene = pipe.scene(&frame);
    (pipe, scene)
}

fn parse_hex(s: &str) -> (u8, u8, u8) {
    let h = s.trim_start_matches('#');
    if h.len() != 6 {
        panic!("expected 6-digit hex, got {s}");
    }
    let v = u32::from_str_radix(h, 16).expect("hex digits");
    ((v >> 16) as u8, ((v >> 8) & 0xff) as u8, (v & 0xff) as u8)
}

fn raster_colors(bytes: &[u8]) -> HashSet<(u8, u8, u8)> {
    let mut reader = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .expect("png header");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    let ch = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("unexpected colour type {other:?}"),
    };
    let w = info.width as usize;
    let h = info.height as usize;
    let stride = info.line_size;
    let mut seen = HashSet::new();
    for row in 0..h {
        let base = row * stride;
        for col in 0..w {
            let o = base + col * ch;
            seen.insert((buf[o], buf[o + 1], buf[o + 2]));
        }
    }
    seen
}

#[test]
fn example_verifies() {
    let (pipe, scene) = example();
    let rep = pipe.verify(&scene);
    assert!(rep.ok(), "geometry failed: {:?}", rep.details());
}

#[test]
fn rules_snap_to_the_pixel_grid() {
    let (_, scene) = example();
    let mut rules = 0;
    for prim in &scene.prims {
        if let Prim::Rule { y, thickness, .. } = prim {
            rules += 1;
            assert_eq!(y.round(), *y, "rule y must be pixel-snapped, got {y}");
            assert_eq!(thickness.round(), *thickness, "thickness must be integral");
        }
    }
    assert!(rules > 0, "example should contain at least one rule");
}

#[test]
fn every_used_palette_token_reaches_the_raster() {
    let (pipe, scene) = example();
    let bytes = render::png(&scene, 1.0, &pipe.fonts).expect("render png");
    let seen = raster_colors(&bytes);
    assert!(
        seen.len() > 20,
        "expected anti-aliased text, got {} colours",
        seen.len()
    );

    let p = &pipe.theme.palette;
    for (name, hex) in [
        ("bg", &p.bg),
        ("surface", &p.surface),
        ("ink", &p.ink),
        ("muted", &p.muted),
        ("accent", &p.accent),
        ("line", &p.line),
        ("canvas.background", &scene.background),
    ] {
        let rgb = parse_hex(hex);
        assert!(seen.contains(&rgb), "{name} = {hex} missing from raster");
    }
}

#[test]
fn scale_is_validated() {
    let (pipe, scene) = example();
    assert!(render::png(&scene, 0.0, &pipe.fonts).is_err());
    assert!(render::png(&scene, -1.0, &pipe.fonts).is_err());
    assert!(render::png(&scene, f32::NAN, &pipe.fonts).is_err());
    assert!(render::png(&scene, f32::INFINITY, &pipe.fonts).is_err());
}
