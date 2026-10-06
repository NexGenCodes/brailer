use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct Face {
    pub upem: f32,
    pub widths: HashMap<char, u16>,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct FontBook {
    faces: HashMap<(String, u16), Arc<Face>>,
    pub fallback: Option<(String, u16)>,
}

const CANDIDATES: &[(&str, u16, &[&str])] = &[
    (
        "sans",
        400,
        &[
            "NotoSans-Regular.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
        ],
    ),
    (
        "sans",
        700,
        &[
            "NotoSans-Bold.ttf",
            "DejaVuSans-Bold.ttf",
            "LiberationSans-Bold.ttf",
        ],
    ),
    (
        "serif",
        400,
        &[
            "DejaVuSerif.ttf",
            "NotoSerif-Regular.ttf",
            "LiberationSerif-Regular.ttf",
        ],
    ),
    (
        "serif",
        700,
        &[
            "DejaVuSerif-Bold.ttf",
            "NotoSerif-Bold.ttf",
            "LiberationSerif-Bold.ttf",
        ],
    ),
];

impl FontBook {
    pub fn discover(roots: &[PathBuf]) -> FontBook {
        let mut book = FontBook::default();
        let mut found: Vec<PathBuf> = Vec::new();
        for root in roots {
            collect(root, &mut found, 0);
        }
        for (family, weight, names) in CANDIDATES {
            let key = (family.to_string(), *weight);
            if book.faces.contains_key(&key) {
                continue;
            }
            for base in *names {
                for path in &found {
                    if path.file_name().and_then(|s| s.to_str()) != Some(base) {
                        continue;
                    }
                    if let Some(face) = load(path) {
                        book.faces.insert(key.clone(), Arc::new(face));
                        break;
                    }
                }
                if book.faces.contains_key(&key) {
                    break;
                }
            }
        }
        book.fallback = book
            .faces
            .get(&("sans".to_string(), 400))
            .map(|_| ("sans".to_string(), 400u16))
            .or_else(|| book.faces.keys().next().cloned());
        book
    }

    pub fn default_roots() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
                .join(".local/share/fonts"),
            PathBuf::from("/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("C:/Windows/Fonts"),
        ]
    }

    fn face(&self, family: &str, weight: u16) -> Option<&Arc<Face>> {
        if let Some(f) = self.faces.get(&(family.to_string(), weight)) {
            return Some(f);
        }
        if let Some(f) = self.faces.get(&(family.to_string(), 400)) {
            return Some(f);
        }
        match &self.fallback {
            Some((fam, w)) => self.faces.get(&(fam.clone(), *w)),
            None => None,
        }
    }

    pub fn missing(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (family, weight, names) in CANDIDATES {
            if !self.faces.contains_key(&(family.to_string(), *weight)) {
                out.push(format!("{family}/{weight} (want one of {:?})", names));
            }
        }
        out
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.faces
            .values()
            .map(|f| f.path.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn advance(&self, family: &str, weight: u16, size: f32, text: &str) -> f32 {
        let Some(face) = self.face(family, weight) else {
            return text.chars().count() as f32 * size * 0.55;
        };
        let fallback = face
            .widths
            .get(&' ')
            .copied()
            .unwrap_or_else(|| (face.upem * 0.28) as u16);
        let mut total = 0.0f32;
        for ch in text.chars() {
            total += face.widths.get(&ch).copied().unwrap_or(fallback) as f32;
        }
        total / face.upem * size
    }

    pub fn wrap(
        &self,
        family: &str,
        weight: u16,
        size: f32,
        measure: f32,
        text: &str,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let words: Vec<&str> = para.split_whitespace().collect();
            if words.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut line = String::new();
            for word in words {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if self.advance(family, weight, size, &candidate) <= measure || line.is_empty() {
                    line = candidate;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            if !line.is_empty() {
                lines.push(line);
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 || out.len() > 20000 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out, depth + 1);
        } else if path.extension().and_then(|s| s.to_str()) == Some("ttf") {
            out.push(path);
        }
    }
}

fn load(path: &Path) -> Option<Face> {
    let data = fs::read(path).ok()?;
    let face = ttf_parser::Face::parse(&data, 0).ok()?;
    let mut widths = HashMap::new();
    if let Some(cmap) = face.tables().cmap {
        for sub in cmap.subtables {
            if !sub.is_unicode() {
                continue;
            }
            sub.codepoints(|cp| {
                let Some(ch) = char::from_u32(cp) else { return };
                if let Some(gid) = sub.glyph_index(cp)
                    && let Some(adv) = face.glyph_hor_advance(gid)
                {
                    widths.insert(ch, adv);
                }
            });
        }
    }
    Some(Face {
        upem: face.units_per_em() as f32,
        widths,
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_finds_noto() {
        let mut found = Vec::new();
        collect(Path::new("/usr/share/fonts"), &mut found, 0);
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
            .collect();
        assert!(
            names.iter().any(|n| n == "NotoSans-Regular.ttf"),
            "NotoSans-Regular not collected; total={}",
            names.len()
        );
        assert!(
            load(Path::new(
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"
            ))
            .is_some(),
            "load() returned None for NotoSans-Regular"
        );
    }

    #[test]
    fn discovery_prefers_noto() {
        let book = FontBook::discover(&FontBook::default_roots());
        let files: Vec<String> = book
            .files()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(
            files.iter().any(|f| f.ends_with("/NotoSans-Regular.ttf")),
            "sans/400 must resolve to NotoSans-Regular, got {files:?}"
        );
        assert!(
            files.iter().any(|f| f.ends_with("/NotoSans-Bold.ttf")),
            "sans/700 must resolve to NotoSans-Bold, got {files:?}"
        );
        assert!(book.missing().is_empty(), "missing: {:?}", book.missing());
    }
}
