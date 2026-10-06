use std::path::Path;

use brailer::load;

// The design-loop corpus: specs written by an LLM from a brief (see
// scripts/design_loop.py and docs/briefs/). Every entry here must keep
// converging — if one of these starts failing `load` or `verify`, the
// engine regressed and the "AI writes a spec that brailer proves" claim
// needs a fix, not a deleted test.
#[test]
fn corpus_specs_all_verify_clean() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut specs: Vec<_> = std::fs::read_dir(&dir)
        .expect("tests/corpus dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    specs.sort();
    assert!(
        !specs.is_empty(),
        "corpus is empty — delete the test instead"
    );

    let mut checked = 0;
    for spec in specs {
        let doc = load(&spec).unwrap_or_else(|e| panic!("{} load: {e}", spec.display()));
        let pipeline = brailer::Pipeline::new(&doc.theme)
            .unwrap_or_else(|e| panic!("{} theme: {e}", spec.display()));
        for (name, frame) in &doc.frames_effective() {
            let scene = pipeline.scene(frame);
            let rep = pipeline.verify(&scene);
            assert!(
                rep.out_of_bounds.is_empty() && rep.collisions.is_empty(),
                "{} [{name}]: out_of_bounds={} collisions={}",
                spec.display(),
                rep.out_of_bounds.len(),
                rep.collisions.len(),
            );
            checked += 1;
        }
    }
    assert!(checked >= 8, "expected >= 8 frame checks, got {checked}");
}
