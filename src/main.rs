use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "brailer",
    version,
    about = "Deterministic design compiler: JSON spec in, SVG/PNG out"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Build {
        spec: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    Verify {
        spec: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        frame: Option<String>,
    },
    Render {
        spec: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
        #[arg(long)]
        retina: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        frame: Option<String>,
    },
    Themes,
    Fonts,
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<i32> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Themes => {
            for t in brailer::known_themes() {
                println!("{t}");
            }
            Ok(0)
        }
        Cmd::Fonts => {
            let book = brailer::FontBook::discover(&brailer::FontBook::default_roots());
            println!("discovered:");
            for f in book.files() {
                println!("  {}", f.display());
            }
            let missing = book.missing();
            if missing.is_empty() {
                println!("  all role slots satisfied");
            }
            for m in missing {
                println!("  MISSING {m}");
            }
            let db = brailer::render::fontdb(&book);
            println!("registered:");
            let mut faces: Vec<String> = db
                .faces()
                .map(|f| {
                    let families = f
                        .families
                        .iter()
                        .map(|(s, _)| s.as_str())
                        .collect::<Vec<_>>()
                        .join("/");
                    format!(
                        "{families} weight={:?} style={:?} index={} {:?}",
                        f.weight, f.style, f.index, f.source
                    )
                })
                .collect();
            faces.sort();
            for f in faces {
                println!("  {f}");
            }
            Ok(0)
        }
        Cmd::Build { spec, out } => {
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;
            let frames = pick_frames(&doc, None)?;
            if frames.len() == 1 {
                let out = out.unwrap_or_else(|| spec.with_extension("svg"));
                let (_, frame) = &frames[0];
                let t = Instant::now();
                let scene = pipe.scene(frame);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                pipe.write_svg(&out, &scene)?;
                eprintln!(
                    "svg {ms:.0}ms -> {} ({}x{}, {} prims)",
                    out.display(),
                    scene.width as i64,
                    scene.height as i64,
                    scene.prims.len()
                );
            } else {
                let out = out.unwrap_or_else(|| {
                    spec.parent()
                        .unwrap_or(Path::new("."))
                        .join("out")
                        .join(spec.file_stem().unwrap_or_default())
                });
                std::fs::create_dir_all(&out)?;
                for (name, frame) in &frames {
                    let scene = pipe.scene(frame);
                    let p = out.join(format!("{name}.svg"));
                    pipe.write_svg(&p, &scene)?;
                    eprintln!(
                        "[{name}] svg -> {} ({}x{}, {} prims)",
                        p.display(),
                        scene.width as i64,
                        scene.height as i64,
                        scene.prims.len()
                    );
                }
            }
            Ok(0)
        }
        Cmd::Verify {
            spec,
            json,
            strict,
            frame,
        } => {
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;
            let frames = pick_frames(&doc, frame.as_deref())?;
            let mut any_fail = false;
            if json {
                let mut list = Vec::new();
                for (name, fr) in &frames {
                    let scene = pipe.scene(fr);
                    let rep = pipe.verify(&scene);
                    let mut obj = serde_json::Map::new();
                    obj.insert("frame".into(), name.clone().into());
                    obj.insert("ok".into(), rep.passed(strict).into());
                    obj.insert("geometry_ok".into(), rep.ok().into());
                    obj.insert("prims".into(), rep.prims.into());
                    obj.insert("width".into(), json_f32(scene.width));
                    obj.insert("height".into(), json_f32(scene.height));
                    obj.insert("errors".into(), arr(&rep.errors));
                    obj.insert("out_of_bounds".into(), arr(&rep.out_of_bounds));
                    obj.insert("collisions".into(), arr(&rep.collisions));
                    obj.insert("contrast".into(), arr(&rep.contrast));
                    if !rep.passed(strict) {
                        any_fail = true;
                    }
                    list.push(serde_json::Value::Object(obj));
                }
                if frames.len() == 1 {
                    println!("{}", list[0]);
                } else {
                    println!("{}", serde_json::Value::Array(list));
                }
            } else {
                for (name, fr) in &frames {
                    let scene = pipe.scene(fr);
                    let rep = pipe.verify(&scene);
                    let tag = if frames.len() > 1 {
                        format!("[{name}] ")
                    } else {
                        String::new()
                    };
                    println!("{tag}{}", rep.summary(strict));
                    for d in rep.details() {
                        println!("{tag}  {d}");
                    }
                    if !rep.passed(strict) {
                        any_fail = true;
                    }
                }
            }
            Ok(if any_fail { 1 } else { 0 })
        }
        Cmd::Render {
            spec,
            out,
            scale,
            retina,
            strict,
            frame,
        } => {
            let out_dir = out.unwrap_or_else(|| {
                spec.parent()
                    .unwrap_or(Path::new("."))
                    .join("out")
                    .join(spec.file_stem().unwrap_or_default())
            });
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;
            let frames = pick_frames(&doc, frame.as_deref())?;

            let mut scales = vec![scale];
            if retina && !scales.contains(&2.0) {
                scales.push(2.0);
            }

            let mut jobs: Vec<(String, brailer::Scene)> = Vec::new();
            for (name, fr) in &frames {
                let t = Instant::now();
                let scene = pipe.scene(fr);
                let layout_ms = t.elapsed().as_secs_f64() * 1000.0;

                let rep = pipe.verify(&scene);
                if !rep.passed(strict) {
                    for d in rep.details() {
                        eprintln!("verify [{name}]: {d}");
                    }
                    bail!("refusing to render: verification failed");
                }

                // Contract: verify must never pass if a later render step would
                // then fail. So the gate runs before any file is created, and
                // every requested scale is pre-flighted here — otherwise
                // --retina could error out after design.svg was already written.
                for s in &scales {
                    brailer::render::check_output(scene.width, scene.height, *s)?;
                }
                let _ = layout_ms;
                jobs.push((name.clone(), scene));
            }

            std::fs::create_dir_all(&out_dir)?;
            for (name, scene) in &jobs {
                let stem = if name == "design" { "design" } else { name };
                let svg_path = out_dir.join(format!("{stem}.svg"));
                pipe.write_svg(&svg_path, scene)?;
                for s in &scales {
                    let p = if s.fract() == 0.0 {
                        out_dir.join(format!("{stem}@{}x.png", *s as i64))
                    } else {
                        out_dir.join(format!("{stem}@{}x.png", s))
                    };
                    pipe.write_png(&p, scene, *s)?;
                }
                let label = if name == "design" {
                    String::new()
                } else {
                    format!("[{name}] ")
                };
                eprintln!(
                    "{label}{}x{} · {} prims · verify PASS -> {}",
                    scene.width as i64,
                    scene.height as i64,
                    scene.prims.len(),
                    out_dir.display()
                );
            }
            Ok(0)
        }
    }
}

fn pick_frames(
    doc: &brailer::Document,
    want: Option<&str>,
) -> Result<Vec<(String, brailer::Frame)>> {
    let mut all: Vec<(String, brailer::Frame)> = doc.frames_effective().into_iter().collect();
    if let Some(want) = want {
        all.retain(|(k, _)| k == want);
        if all.is_empty() {
            bail!(
                "no frame named {want:?}; available: {}",
                doc.frames_effective()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(all)
}

fn json_f32(v: f32) -> serde_json::Value {
    serde_json::Number::from_f64(v as f64)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn arr(items: &[String]) -> serde_json::Value {
    serde_json::Value::Array(
        items
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect(),
    )
}
