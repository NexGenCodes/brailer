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
            let out = out.unwrap_or_else(|| spec.with_extension("svg"));
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;
            let t = Instant::now();
            let scene = pipe.scene(&doc);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            pipe.write_svg(&out, &scene)?;
            eprintln!(
                "svg {ms:.0}ms -> {} ({}x{}, {} prims)",
                out.display(),
                scene.width as i64,
                scene.height as i64,
                scene.prims.len()
            );
            Ok(0)
        }
        Cmd::Verify { spec, json, strict } => {
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;
            let scene = pipe.scene(&doc);
            let rep = pipe.verify(&scene);
            if json {
                let mut obj = serde_json::Map::new();
                obj.insert("ok".into(), rep.passed(strict).into());
                obj.insert("geometry_ok".into(), rep.ok().into());
                obj.insert("prims".into(), rep.prims.into());
                obj.insert("width".into(), json_f32(scene.width));
                obj.insert("height".into(), json_f32(scene.height));
                obj.insert("errors".into(), arr(&rep.errors));
                obj.insert("out_of_bounds".into(), arr(&rep.out_of_bounds));
                obj.insert("collisions".into(), arr(&rep.collisions));
                obj.insert("contrast".into(), arr(&rep.contrast));
                println!("{}", serde_json::Value::Object(obj));
            } else {
                println!("{}", rep.summary(strict));
                for d in rep.details() {
                    println!("  {d}");
                }
            }
            Ok(if rep.passed(strict) { 0 } else { 1 })
        }
        Cmd::Render {
            spec,
            out,
            scale,
            retina,
            strict,
        } => {
            let out_dir = out.unwrap_or_else(|| {
                spec.parent()
                    .unwrap_or(Path::new("."))
                    .join("out")
                    .join(spec.file_stem().unwrap_or_default())
            });
            let doc = brailer::load(&spec)?;
            let pipe = brailer::Pipeline::new(&doc.theme)?;

            let t = Instant::now();
            let scene = pipe.scene(&doc);
            let layout_ms = t.elapsed().as_secs_f64() * 1000.0;

            let rep = pipe.verify(&scene);
            if !rep.passed(strict) {
                for d in rep.details() {
                    eprintln!("verify: {d}");
                }
                bail!("refusing to render: verification failed");
            }

            let mut scales = vec![scale];
            if retina && !scales.contains(&2.0) {
                scales.push(2.0);
            }
            for s in &scales {
                brailer::render::check_output(scene.width, scene.height, *s)?;
            }

            std::fs::create_dir_all(&out_dir).ok();
            let svg_path = out_dir.join("design.svg");
            pipe.write_svg(&svg_path, &scene)?;
            let svg_ms = t.elapsed().as_secs_f64() * 1000.0 - layout_ms;

            for s in scales {
                let p = if s.fract() == 0.0 {
                    out_dir.join(format!("design@{}x.png", s as i64))
                } else {
                    out_dir.join(format!("design@{}x.png", s))
                };
                pipe.write_png(&p, &scene, s)?;
            }
            let total = t.elapsed().as_secs_f64() * 1000.0;
            eprintln!(
                "layout {layout_ms:.0}ms · svg {svg_ms:.0}ms · total {total:.0}ms · {}x{} · {} prims · verify PASS",
                scene.width as i64,
                scene.height as i64,
                scene.prims.len()
            );
            eprintln!("-> {}", out_dir.display());
            Ok(0)
        }
    }
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
