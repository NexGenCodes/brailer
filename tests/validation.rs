use brailer::spec;

fn spec_of(json: &str) -> spec::Document {
    serde_json::from_str(json).expect("spec should parse as json")
}

fn validation_errors(json: &str) -> Vec<String> {
    spec::validate(&spec_of(json))
}

fn one_error_containing(json: &str, needle: &str) {
    let errs = validation_errors(json);
    assert!(
        errs.iter().any(|e| e.contains(needle)),
        "expected an error containing {needle:?}, got {errs:?}"
    );
}

const W: &str = r##"{"canvas":{"width":1440,"background":"#FAF7F2"},"theme":"editorial","root":"##;

fn stack(children: &str) -> String {
    format!("{W}{{\"kind\":\"stack\",\"children\":[{children}]}}}}")
}

#[test]
fn accepts_the_shipped_example() {
    let doc = brailer::load(std::path::Path::new("examples/editorial.json")).expect("example");
    assert!(spec::validate(&doc).is_empty());
}

#[test]
fn rejects_zero_columns_that_used_to_panic() {
    one_error_containing(
        &stack(r##"{"kind":"grid","columns":0,"children":[]}"##),
        "columns must be 1..=",
    );
    one_error_containing(
        &stack(&format!(
            r##"{{"kind":"grid","columns":{},"children":[]}}"##,
            spec::MAX_COLUMNS + 1
        )),
        "columns must be 1..=",
    );
    assert!(
        validation_errors(&stack(&format!(
            r##"{{"kind":"grid","columns":{},"children":[]}}"##,
            spec::MAX_COLUMNS
        )))
        .is_empty(),
        "MAX_COLUMNS itself must be accepted"
    );
}

#[test]
fn rejects_negative_and_non_finite_layout_values() {
    one_error_containing(
        &stack(r##"{"kind":"stack","gap":-50,"children":[]}"##),
        "gap must be",
    );
    one_error_containing(
        &stack(r##"{"kind":"stack","pad":-1,"children":[]}"##),
        "pad must be",
    );
    one_error_containing(
        &stack(r##"{"kind":"spacer","height":-5}"##),
        "spacer height",
    );
    one_error_containing(&stack(r##"{"kind":"rule","thickness":0}"##), "thickness");
    one_error_containing(
        &stack(r##"{"kind":"card","radius":-2,"children":[]}"##),
        "radius",
    );
}

#[test]
fn rejects_invalid_colours_including_multibyte_input() {
    one_error_containing(
        &stack(r##"{"kind":"text","text":"hi","role":"label","color":"notacolor"}"##),
        "not a valid colour",
    );
    one_error_containing(
        &stack(r##"{"kind":"text","text":"hi","role":"label","color":"#zzzzzz"}"##),
        "not a valid colour",
    );
    one_error_containing(
        &stack(r##"{"kind":"text","text":"hi","role":"label","color":"#aé"}"##),
        "not a valid colour",
    );
    let bad_canvas = r###"{"canvas":{"width":1440,"background":"##"},"theme":"editorial","root":{"kind":"stack","children":[]}}"###;
    one_error_containing(bad_canvas, "canvas.background");
}

#[test]
fn rejects_oversized_and_degenerate_input() {
    let narrow = r##"{"canvas":{"width":10,"background":"#fff"},"theme":"editorial","root":{"kind":"stack","children":[]}}"##;
    one_error_containing(narrow, "canvas.width");

    let many = (0..spec::MAX_LEAVES + 1)
        .map(|_| r##"{"kind":"spacer","height":1}"##)
        .collect::<Vec<_>>()
        .join(",");
    one_error_containing(&stack(&many), "leaf nodes");

    one_error_containing(
        &stack(r##"{"kind":"text","text":"   ","role":"body"}"##),
        "text node is empty",
    );
    one_error_containing(
        &stack(r##"{"kind":"text","text":"word","role":"body","max_measure":0}"##),
        "max_measure",
    );
}

#[test]
fn rejects_dangerous_raw_svg() {
    one_error_containing(
        &stack(r##"{"kind":"raw","svg":"<script>alert(1)</script>","height":10}"##),
        "<script is not allowed",
    );
    one_error_containing(
        &stack(
            r##"{"kind":"raw","svg":"<!DOCTYPE svg [<!ENTITY x SYSTEM \"file:///etc/passwd\">]>","height":10}"##,
        ),
        "doctype/entity",
    );
    one_error_containing(
        &stack(r##"{"kind":"raw","svg":"<a href=\"javascript:alert(1)\">x</a>","height":10}"##),
        "javascript:",
    );
}

#[test]
fn parse_hex_never_panics_and_rejects_garbage() {
    for s in [
        "#aé",
        "ééé",
        "#",
        "",
        "##",
        "#12",
        "#1234567",
        "#zzzzzz",
        "rgb(1,2,3)",
        "😀😀😀",
    ] {
        assert!(
            brailer::verify::parse_hex(s).is_none(),
            "expected {s:?} to be rejected"
        );
    }
    assert_eq!(brailer::verify::parse_hex("#fff"), Some((1.0, 1.0, 1.0)));
    assert_eq!(brailer::verify::parse_hex("#000000"), Some((0.0, 0.0, 0.0)));
    assert_eq!(
        brailer::verify::parse_hex("a0492b"),
        Some((160.0 / 255.0, 73.0 / 255.0, 43.0 / 255.0))
    );
}

#[test]
fn verify_catches_malformed_raw_and_oversized_canvas() {
    let doc = spec_of(&stack(
        r##"{"kind":"raw","svg":"<not valid <<<<","height":50}"##,
    ));
    let pipe = brailer::Pipeline::new(&doc.theme).expect("theme");
    let frame = doc
        .frames_effective()
        .remove("design")
        .expect("a design frame");
    let scene = pipe.scene(&frame);
    let rep = brailer::verify::scene(&scene, 0.75);
    assert!(!rep.ok(), "malformed raw svg must fail verification");
    assert!(
        rep.errors.iter().any(|e| e.contains("raw svg")),
        "got {:?}",
        rep.errors
    );
}

#[test]
fn verify_rejects_canvas_beyond_the_raster_limit() {
    let scene = brailer::Scene {
        width: 1440.0,
        height: 500_000.0,
        background: "#fff".into(),
        prims: Vec::new(),
    };
    let rep = brailer::verify::scene(&scene, 0.0);
    assert!(!rep.ok(), "oversized canvas must fail verification");
    assert!(
        rep.errors.iter().any(|e| e.contains("raster limit")),
        "got {:?}",
        rep.errors
    );
}

#[test]
fn verify_flags_non_finite_geometry() {
    let scene = brailer::Scene {
        width: 1440.0,
        height: 100.0,
        background: "#fff".into(),
        prims: Vec::new(),
    };
    let rep = brailer::verify::scene(&scene, 0.0);
    assert!(rep.ok(), "clean scene should pass: {:?}", rep.errors);

    let mut nan = scene.clone();
    nan.width = f32::NAN;
    assert!(!brailer::verify::scene(&nan, 0.0).ok());
}

fn text_prim(x: f32, y: f32, w: f32, h: f32) -> brailer::primitive::Prim {
    brailer::primitive::Prim::Text {
        x,
        y,
        w,
        h,
        text: "sample".into(),
        size: 16.0,
        family: "sans".into(),
        weight: 400,
        fill: "#14110F".into(),
        align: brailer::spec::Align::Start,
        tracking: 0.0,
    }
}

fn scene_of(prims: Vec<brailer::primitive::Prim>) -> brailer::Scene {
    brailer::Scene {
        width: 1440.0,
        height: 3000.0,
        background: "#fff".into(),
        prims,
    }
}

#[test]
fn collision_sweep_detects_overlaps() {
    let rep = brailer::verify::scene(
        &scene_of(vec![
            text_prim(10.0, 50.0, 200.0, 30.0),
            text_prim(10.0, 60.0, 200.0, 30.0),
        ]),
        0.0,
    );
    assert_eq!(rep.collisions.len(), 1, "expected 1 collision: {:?}", rep);
    assert!(!rep.ok());
}

#[test]
fn collision_sweep_reports_no_false_positives() {
    let rep = brailer::verify::scene(
        &scene_of(vec![
            text_prim(10.0, 50.0, 200.0, 30.0),
            text_prim(10.0, 200.0, 200.0, 30.0),
            text_prim(500.0, 50.0, 200.0, 30.0),
            text_prim(1400.0, 55.0, 30.0, 30.0),
        ]),
        0.0,
    );
    assert!(
        rep.collisions.is_empty(),
        "false positive: {:?}",
        rep.collisions
    );
    assert!(rep.ok(), "{:?}", rep.errors);
}

#[test]
fn output_size_is_validated() {
    let (w, h) = brailer::render::check_output(1440.0, 8000.0, 2.0).unwrap();
    assert_eq!((w, h), (2880, 16000));
    assert!(brailer::render::check_output(1440.0, 9000.0, 2.0).is_err());
    assert!(brailer::render::check_output(1440.0, 900.0, 0.0).is_err());
    assert!(brailer::render::check_output(1440.0, 900.0, f32::NAN).is_err());
    assert!(brailer::render::check_output(1440.0, 900.0, -1.0).is_err());
    assert!(brailer::render::check_output(0.0, 900.0, 1.0).is_err());
}

#[test]
fn render_pre_flights_scale_before_writing_anything() {
    let dir = std::env::temp_dir().join(format!("brailer-preflight-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cards: Vec<String> = (0..300)
        .map(|i| {
            format!(
                r##"{{"kind":"text","text":"Row {i} — oak brass linen.","role":"body","max_measure":1360}}"##
            )
        })
        .collect();
    let json = format!(
        r##"{{"canvas":{{"width":1440,"background":"#FAF7F2"}},"theme":"editorial","root":{{"kind":"stack","gap":6,"pad":40,"children":[{}]}}}}"##,
        cards.join(",")
    );
    let spec = dir.join("tall.json");
    std::fs::write(&spec, json).unwrap();
    let out = dir.join("out");

    let res = std::process::Command::new(env!("CARGO_BIN_EXE_brailer"))
        .args([
            "render",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--scale",
            "1",
            "--retina",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&res.stderr);
    assert!(!res.status.success(), "expected a failure: {stderr}");
    assert!(
        !stderr.contains("verification failed"),
        "verify must pass before the pre-flight fires: {stderr}"
    );
    assert!(stderr.contains("lower --scale"), "{stderr}");
    assert!(
        !out.exists(),
        "nothing should be written when pre-flight fails"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn every_builtin_theme_passes_contrast() {
    for name in brailer::known_themes() {
        let theme = brailer::theme::builtin(name).unwrap_or_else(|| panic!("unknown theme {name}"));
        let issues = brailer::verify::theme(&theme);
        assert!(issues.is_empty(), "theme {name}: {issues:?}");
    }
}
