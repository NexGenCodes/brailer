use std::path::PathBuf;

use brailer::primitive::Prim;
use brailer::spec;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("brailer-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn scene_of(json: &str) -> (brailer::Pipeline, brailer::Scene) {
    let doc: spec::Document = serde_json::from_str(json).expect("parse");
    assert!(spec::validate(&doc).is_empty(), "doc must validate");
    let pipe = brailer::Pipeline::new(&doc.theme).expect("theme");
    let frame = doc.frames_effective().remove("design").unwrap();
    let scene = pipe.scene(&frame);
    (pipe, scene)
}

fn text_prims(scene: &brailer::Scene) -> Vec<&Prim> {
    scene
        .prims
        .iter()
        .filter(|p| matches!(p, Prim::Text { .. }))
        .collect()
}

#[test]
fn span_cells_flow_over_shared_rows() {
    let json = r##"{"canvas":{"width":1440,"background":"#FFFFFF"},"theme":"canary","root":{"kind":"grid","columns":4,"gap":16,"pad":32,"children":[
        {"kind":"cell","span":2,"child":{"kind":"text","text":"wide","role":"h1"}},
        {"kind":"text","text":"a","role":"body"},
        {"kind":"text","text":"b","role":"body"},
        {"kind":"text","text":"c","role":"body"},
        {"kind":"text","text":"d","role":"body"}
    ]}}"##;
    let (_, scene) = scene_of(json);
    // inner = 1440 - 64 - 48 = 1328 → col_w = 332; a span-2 cell spans 2*332 + 16.
    let find = |needle: &str| {
        text_prims(&scene)
            .into_iter()
            .find(|p| matches!(p, Prim::Text { text, .. } if text == needle))
            .unwrap_or_else(|| panic!("{needle} missing"))
    };
    let (x, y) = match find("wide") {
        Prim::Text { x, y, .. } => (*x, *y),
        _ => unreachable!(),
    };
    assert_eq!(x, 32.0, "span-2 cell starts at the grid pad");
    let xy = |needle: &str| match find(needle) {
        Prim::Text { x, y, .. } => (*x, *y),
        _ => unreachable!(),
    };
    // Row 1 continues past the span at column 2 (32 + 2*348).
    let (ax, ay) = xy("a");
    assert_eq!(ax, 728.0);
    let (bx, _) = xy("b");
    assert_eq!(bx, 1076.0);
    // Row 2 wraps: "c" reuses the left slot, "d" packs beside it.
    let (cx, cy) = xy("c");
    let (dx, _) = xy("d");
    assert_eq!(cx, 32.0);
    assert_eq!(dx, 380.0, "d packs into the column next to c");
    assert!(cy > ay, "row 2 must sit below row 1");
    let (wy, _) = (y, x);
    assert!(cy > wy, "row 2 must clear the span row");
}

#[test]
fn cell_outside_a_grid_is_rejected() {
    let json = r##"{"canvas":{"width":400,"background":"#fff"},"theme":"canary","root":{"kind":"stack","children":[{"kind":"cell","span":2,"child":{"kind":"spacer","height":4}}]}}"##;
    let doc: spec::Document = serde_json::from_str(json).unwrap();
    let errs = spec::validate(&doc);
    assert!(
        errs.iter().any(|e| e.contains("direct child of a grid")),
        "got {errs:?}"
    );
}

#[test]
fn image_src_resolves_against_asset_dir_and_emits_fit() {
    let dir = tmpdir("img");
    let assets = dir.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, 2, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[0, 0, 0, 0, 10, 20, 30, 255, 40, 50, 60, 255, 255, 0, 0, 255])
            .unwrap();
    }
    std::fs::write(assets.join("photo.png"), &buf).unwrap();
    let spec = dir.join("s.json");
    std::fs::write(
        &spec,
        r##"{"theme":"canary","asset_dir":"assets","canvas":{"width":600,"background":"#fff"},"root":{"kind":"stack","gap":8,"children":[
            {"kind":"image","src":"photo.png","height":300},
            {"kind":"image","src":"photo.png","width":400,"height":200,"fit":"contain"},
            {"kind":"image","src":"photo.png","width":300,"height":120,"fit":"fill"}
        ]}}"##,
    )
    .unwrap();

    let doc = brailer::load(&spec).expect("load resolves relative image");
    let pipe = brailer::Pipeline::new(&doc.theme).unwrap();
    let frame = doc.frames_effective().remove("design").unwrap();
    let scene = pipe.scene(&frame);
    let imgs: Vec<&Prim> = scene
        .prims
        .iter()
        .filter(|p| matches!(p, Prim::Image { .. }))
        .collect();
    assert_eq!(imgs.len(), 3);
    for p in &imgs {
        let (href, fit) = match p {
            Prim::Image { href, fit, .. } => (href, *fit),
            _ => unreachable!(),
        };
        assert!(
            href.starts_with("data:image/png;base64,"),
            "base64 data uri"
        );
        assert!(match fit {
            spec::Fit::Cover | spec::Fit::Contain | spec::Fit::Fill => true,
        });
    }
    let svg = brailer::svg::render(&scene);
    assert!(svg.contains("xMidYMid slice"), "default cover crops");
    assert!(svg.contains("xMidYMid meet"), "contain letterboxes");
    assert!(
        svg.contains("preserveAspectRatio=\"none\""),
        "fill stretches"
    );
}

#[test]
fn unresolved_image_src_is_a_load_error() {
    let dir = tmpdir("imgmiss");
    let spec = dir.join("s.json");
    std::fs::write(
        &spec,
        r##"{"theme":"canary","asset_dir":"assets","canvas":{"width":600,"background":"#fff"},"root":{"kind":"image","src":"nope.png","height":10}}"##,
    )
    .unwrap();
    let err = brailer::load(&spec).unwrap_err();
    assert!(format!("{err:#}").contains("not resolvable"), "{err:#}");
}

#[test]
fn components_inline_into_the_tree_once() {
    let json = r##"{"theme":"canary","components":{
        "chip":{"kind":"card","children":[{"kind":"text","text":"hi","role":"label"}]},
        "nested":{"kind":"stack","children":[{"kind":"component","name":"chip"}]}
    },"canvas":{"width":400,"background":"#fff"},"root":{"kind":"stack","children":[{"kind":"component","name":"nested"}]}}"##;
    let mut doc: spec::Document = serde_json::from_str(json).unwrap();
    let mut errors = Vec::new();
    spec::expand_components(&mut doc, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(spec::validate(&doc).is_empty());

    let pipe = brailer::Pipeline::new(&doc.theme).unwrap();
    let frame = doc.frames_effective().remove("design").unwrap();
    let scene = pipe.scene(&frame);
    assert!(
        text_prims(&scene)
            .iter()
            .any(|p| matches!(p, Prim::Text { text, .. } if text == "hi")),
        "inlined component text must reach the scene"
    );
    assert!(
        text_prims(&scene)
            .iter()
            .all(|p| !matches!(p, Prim::Raw { .. })),
        "no raw prims should appear"
    );
}

#[test]
fn component_cycles_are_rejected() {
    let json = r##"{"theme":"canary","components":{
        "a":{"kind":"stack","children":[{"kind":"component","name":"b"}]},
        "b":{"kind":"stack","children":[{"kind":"component","name":"a"}]}
    },"canvas":{"width":400,"background":"#fff"},"root":{"kind":"component","name":"a"}}"##;
    let mut doc: spec::Document = serde_json::from_str(json).unwrap();
    let mut errors = Vec::new();
    spec::expand_components(&mut doc, &mut errors);
    assert!(
        errors.iter().any(|e| e.contains("cycle")),
        "expected a cycle error, got {errors:?}"
    );
}

#[test]
fn tracking_emits_letter_spacing() {
    let json = r##"{"canvas":{"width":400,"background":"#fff"},"theme":"canary","root":{"kind":"stack","children":[{"kind":"text","text":"Spread","role":"h1","tracking":0.5}]}}"##;
    let (_, scene) = scene_of(json);
    let svg = brailer::svg::render(&scene);
    assert!(svg.contains("letter-spacing="), "{svg}");
}

#[test]
fn gradients_and_shadows_reach_svg_defs() {
    let json = r##"{"canvas":{"width":400,"background":"#fff"},"theme":"canary","root":{"kind":"card","bg":{"from":"#F7D24B","to":"#B3261E","angle":90},"shadow":{"blur":8,"x":0,"y":2},"radius":12,"children":[{"kind":"spacer","height":60}]}}"##;
    let (_, scene) = scene_of(json);
    let svg = brailer::svg::render(&scene);
    assert!(svg.contains("<linearGradient"), "gradient def");
    assert!(svg.contains("<feDropShadow"), "shadow def");
    assert!(svg.contains("url(#g0)"), "card references gradient");
    assert!(
        svg.contains("filter=\"url(#f0)\""),
        "card references shadow"
    );
    assert!(
        svg.contains("flood-color=\"#17161A24\""),
        "default shadow tint"
    );
}

#[test]
fn frames_render_multiple_scenes_and_filter_by_name() {
    let dir = tmpdir("frames");
    let spec = dir.join("f.json");
    std::fs::write(
        &spec,
        r##"{"theme":"canary","frames":{
            "mob":{"canvas":{"width":390,"background":"#fff"},"root":{"kind":"text","text":"m","role":"h1"}},
            "desk":{"canvas":{"width":1440,"background":"#fff"},"root":{"kind":"text","text":"d","role":"h1"}}
        }}"##,
    )
    .unwrap();

    let doc = brailer::load(&spec).unwrap();
    assert_eq!(doc.frames_effective().len(), 2);
    let pipe = brailer::Pipeline::new(&doc.theme).unwrap();
    let frames = doc.frames_effective();
    let mob = pipe.scene(&frames["mob"]);
    let desk = pipe.scene(&frames["desk"]);
    assert_eq!(mob.width, 390.0);
    assert_eq!(desk.width, 1440.0);
    assert!(pipe.verify(&mob).ok());
    assert!(pipe.verify(&desk).ok());

    let out = dir.join("out");
    let res = std::process::Command::new(env!("CARGO_BIN_EXE_brailer"))
        .args([
            "render",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        res.status.success(),
        "{}",
        String::from_utf8_lossy(&res.stderr)
    );
    assert!(out.join("mob@1x.png").exists());
    assert!(out.join("desk@1x.png").exists());
    assert!(out.join("mob.svg").exists());
    assert!(out.join("desk.svg").exists());
    assert!(!out.join("design.svg").exists());

    let only = dir.join("only");
    let res = std::process::Command::new(env!("CARGO_BIN_EXE_brailer"))
        .args([
            "render",
            spec.to_str().unwrap(),
            "--out",
            only.to_str().unwrap(),
            "--frame",
            "mob",
        ])
        .output()
        .unwrap();
    assert!(
        res.status.success(),
        "{}",
        String::from_utf8_lossy(&res.stderr)
    );
    assert!(only.join("mob@1x.png").exists());
    assert!(!only.join("desk@1x.png").exists());
}
