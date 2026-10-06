# brailer

**A design compiler. Semantic JSON in, SVG/PNG out — no browser, no Inkscape, no headless Chrome.**

```
design.json  ──▶  brailer  ──▶  design.svg
                                 design@1x.png
                                 design@2x.png
```

`brailer` exists because LLMs are bad at emitting geometry and bad at seeing pixels.
They *are* good at meaning. So the split is:

| you emit | the engine owns |
|---|---|
| `role: "h1"`, `gap: 24`, `columns: 3` | every coordinate, size, family, colour, leading |
| structure and hierarchy | text measurement, wrapping, collisions, contrast |

No hand-written SVG. No CSS. No layout in the prompt.

It targets the screens an AI agent is actually asked to produce: **auth**, **home**,
**checkout**, plus landing pages, product cards, posters and social graphics.

## Why Rust

Because `resvg` is pure Rust. That single fact decides everything else:

- **No non-Rust code in the binary.** Not one line of C, C++ or Go.
- **Single static binary, under 5 MB.** Drop it anywhere; it does not care about the OS.
- **Zero runtime dependencies.** No X11, no fontconfig, no Inkscape, no browser.
- Go cannot reach `resvg` without cgo, which breaks every one of those guarantees.

Rasterisation is deterministic **given the same fonts** — shaping and rendering
all happen inside one crate, but fonts are discovered from the local OS, so two
machines with different installed fonts will not produce identical output.

## Install

**From source** (works today):

```sh
cargo install --path .
# or, for a debug-free optimised build
cargo build --release   # target/release/brailer
```

**Coming with the first release** (source tarball + CI-built binaries):

```sh
brew install brailer
cargo binstall brailer
curl -fsSL https://github.com/NexGenCodes/brailer/releases/latest/install.sh | sh
```

> The Homebrew formula, release binaries and install script are planned, not yet
> published. Until then, use `cargo install`.

## Quickstart

```sh
brailer themes                       # canary · editorial
brailer fonts                        # discovery + what resvg actually registered

cat > /tmp/first.json <<'JSON'
{ "canvas": { "width": 1440 },
  "theme": "canary",
  "root": { "kind": "stack", "gap": 16, "pad": 48,
    "children": [
      { "kind": "text", "role": "display", "text": "Cedar & Lead" },
      { "kind": "text", "role": "body", "text": "Pencils, sharpened daily." } ] } }
JSON

brailer verify /tmp/first.json          # prims=2 ... -- PASS
brailer render /tmp/first.json -o out/  # -> out/design@1x.png, out/design.svg
```

Exit codes are the contract: `0` passed, `1` verification failed, `2` rejected
before layout (invalid spec). Nothing ever panics on hostile input.

## The spec

```jsonc
{
  "canvas": { "width": 1440, "background": "#FAF7F2" },
  "theme": "editorial",
  "root": {
    "kind": "stack", "gap": 24, "pad": 48,
    "children": [
      { "kind": "text", "role": "h1", "text": "Solid ash, hand-planed" },
      { "kind": "text", "role": "body", "max_measure": 620,
        "text": "A seat three, without a central leg." },
      { "kind": "rule" },
      { "kind": "grid", "columns": 3, "gap": 24, "children": [ /* cards */ ] }
    ]
  }
}
```

Ten node types, deliberately small and compositional:

`stack` · `grid` · `cell` · `text` · `image` · `rule` · `spacer` · `card` · `raw` · `component`

Sections are *presets you compose*, never an enum of page types — so a new
layout is a new spec, not a new release.

Text carries a **role**, not a size (sizes shown for `canary`):

`display` (64) · `h1` (40) · `h2` (28) · `h3` (19) · `body` (15) · `small` (13) · `label` (11)

The role picks family (serif for display/headings, sans below) and colour
(ink → muted → accent) from the theme. That is where consistency comes from.

A spec can ship **responsive frames** — desktop **and** mobile — in one file:

```jsonc
{
  "theme": "canary",
  "frames": {
    "desktop": { "canvas": { "width": 1440, "background": "#FBF9F3" }, "root": { /* ... */ } },
    "mobile":  { "canvas": { "width": 390,  "background": "#FBF9F3" }, "root": { /* ... */ } }
  }
}
```

Legacy `canvas`+`root` specs still work and become a single `design` frame.
`verify` and `render` loop over every frame; `--frame NAME` targets one.

The full reference for agent platforms is [`SKILL.md`](SKILL.md).

## Images

First-class `image` node — photos are content, not escape hatches:

```jsonc
{ "kind": "image", "src": "photo.jpg", "width": 420, "height": 260, "fit": "cover" }
```

- `src` is a `data:` URI or a path resolved at load time against `asset_dir`
  (relative to the spec's folder). SVGs are emitted self-contained.
- `fit`: `cover` (crop to fill, default) · `contain` (letterbox) · `fill`
  (stretch) — mapped to the matching SVG `preserveAspectRatio`.

Verified: base64 JPEG/PNG/WebP and resolved files land in the raster with
pixels intact; a panelled marketplace home (12 photos, `cover`) renders at
1440 px with every product image present.

Other visual depth available in the spec: two-stop **linear gradients** as
`bg` (`{ "from", "to", "angle" }`), `shadow` on containers (blur/x/y/color),
and `tracking` (letter-spacing in em) on any `text`.

## What it costs

**Money: nothing.** No API keys, no cloud service, no licence, no per-render
charge. Everything runs on your machine, offline. The dependencies are all
permissive-licensed crates (`resvg`/`usvg` are MPL-2.0-or-Apache, the rest MIT/
Apache-2.0).

**Disk:** 6.2 MB binary, linking only `libm`, `libgcc_s` and the C runtime — no
X11, no fontconfig, no browser.

**Time:** ~310 ms for a typical 1440×1087 page (1× + 2×), ~1 s for a full-length
(≈3,300 px) page — measured on one development machine, not a CI benchmark.
Layout and SVG generation are 1–3 ms; rasterisation is effectively the whole
cost. Re-run `python3 scripts/bench.py` on your hardware to get your own numbers.

**Memory:** 158 MB peak for that full page at 1× + 2× (two rasters held at
once). A single 1440×1087 render is ~10 MB. The 16,000 px output cap is what
keeps this bounded — a pathological 16,000² canvas would want ~1 GB.

**What it replaces:** a headless Chromium or Inkscape install (100–300 MB, a
process spawn per render, and a dependency you have to provision on every
machine) — and it is faster than both.

## Verification

`brailer verify` is a real gate, not a linter. It reports four classes:

| check | meaning | severity |
|---|---|---|
| `errors` | bad colour, negative gap, empty text, unsafe SVG | fail |
| `out_of_bounds` | geometry off the canvas | fail |
| `collisions` | two text runs overlap | fail |
| `contrast` | below 4.5:1 | warn (`--strict` → fail) |

`brailer render` runs the same gate and **refuses to render on failure**, then
pre-flights the requested `--scale` so it cannot fail *after* you were told
everything passed. `verify --json` gives the same numbers for a fix loop.

## Guarantees

- **No panics on hostile input.** 27 adversarial specs (zero columns, multibyte
  hex colours, `NaN` widths, XXE in `raw`, 40k-node explosions) all reject
  cleanly with messages.
- **Bounded cost.** Nesting ≤ 64, leaves ≤ 20,000, output ≤ 16,000 px, collision
  sweep budgeted — every limit is a named constant, not a magic number.
- **Safe `raw`.** `<script>`, doctype/entity declarations, `javascript:` and
  `data:text/html` are rejected at parse time.
- **Crisp output.** Rules snap to the pixel grid; font selection is verified by
  decoding the PNG and asserting every used palette colour actually landed in it.

Measured on a 1440×1087, 30-primitive spec on one dev machine: **verify ~10 ms,
render ~310 ms** (layout 1 ms, SVG 1 ms, raster 310 ms). A 12,801-primitive
stress spec verifies in **80 ms**. These are not CI benchmarks — run
`python3 scripts/bench.py` to re-measure on your hardware.

## Architecture

```
spec.rs     schema + validation (rejects before layout)
fonts.rs    system font discovery, priority-ordered per role slot
layout.rs   spec -> Prims (all geometry resolved here)
theme.rs    palette, type scale, spacing, radius
svg.rs      Prims -> SVG
render.rs   SVG -> PNG via resvg + tiny-skia
verify.rs   geometric, structural and contrast report
main.rs     CLI
```

Layout is infallible by design: invalid input never reaches it. Validation
`bails` first with a list of every problem found, not just the first.

## Honest limitations

- **You cannot see the output.** Verification is structural, geometric and
  pixel-statistical. Aesthetic review needs a human or a vision model.
- **Text shaping is simple.** The renderer is resvg, not Chromium: no CSS, no
  RTL bidi, no complex-script shaping, no CJK-aware line breaking, no
  hyphenation. Latin-language design is solid; multilingual design is a real
  gap today.
- **Fonts come from the OS.** Output depends on what the local system has
  installed; the same spec renders differently on machines with different
  fonts. Pin a font set in CI if identical output matters.
- **Responsive is hand-authored, not automatic.** Desktop and mobile frames
  are both authored in the spec; the engine does not reflow a layout
  automatically. Same content, two compositions — that shared content can and
  should live in `components`.
- **Bespoke art direction is not automatable.** `raw` is an escape hatch, not a
  creative eye. Engine-owned layout, typography, colour and composition cover
  most of what makes a page work; the rest does not reduce to rules.
- **Two themes ship** (`editorial`, `canary`) — the palette, type scale and
  spacing tokens that typify an editorial brand and an ecommerce store. More
  themes are the highest-leverage next feature.

## Licence

Licensed under the MIT Licence (permissive — use it in anything, commercial or not).
