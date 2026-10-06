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

## Why Rust

Because `resvg` is pure Rust. That single fact decides everything else:

- **No non-Rust code in the binary.** Not one line of C, C++ or Go.
- **Single static binary, under 5 MB.** Drop it anywhere; it does not care about the OS.
- **Zero runtime dependencies.** No X11, no fontconfig, no Inkscape, no browser.
- Go cannot reach `resvg` without cgo, which breaks every one of those guarantees.

Rasterisation is deterministic and identical on every machine, because the font
discovery, shaping and rendering all happen inside one crate.

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
brailer themes                       # editorial
brailer fonts                        # discovery + what resvg actually registered

brailer verify examples/editorial.json
# prims=30 size=1440x1087 errors=0 out_of_bounds=0 collisions=0 contrast=0 -- PASS

brailer render examples/editorial.json --retina
# layout 1ms · svg 1ms · total 312ms · verify PASS
# -> examples/out/editorial
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

Seven node kinds, deliberately small and compositional:

`stack` · `grid` · `text` · `rule` · `spacer` · `card` · `raw`

Sections are *presets you compose*, never an enum of page types — so a new
layout is a new spec, not a new release.

Text carries a **role**, not a size:

`display` (76) · `h1` (48) · `h2` (32) · `h3` (21) · `body` (16) · `small` (13) · `label` (12)

The role picks family (serif for display/headings, sans below) and colour
(ink → muted → accent) from the theme. That is where consistency comes from.

The full reference for agent platforms is [`SKILL.md`](SKILL.md).

## Images and logos

Yes — images work today, through the `raw` node. Embed the file as a data URI:

```jsonc
{ "kind": "raw", "height": 160,
  "svg": "<image href=\"data:image/png;base64,iVBORw0KGgo...\" width=\"160\" height=\"160\"/>" }
```

Verified: a 64×64 PNG embedded this way lands in the raster with its pixels
intact — a 120×120 placement produced 14,280 matching pixels against 14,400
expected, the shortfall being antialiased edge pixels. The same works for JPEG, GIF, WebP and nested SVG.

What this means in practice:

| want | how |
|---|---|
| a product photo in a flyer | embed it as a data URI in `raw` |
| your brand logo | embed the SVG, or inline its paths in `raw` |
| bespoke illustration | `raw` with any SVG fragment |

**Known gaps.** There is no first-class `image` node yet, so you must base64 the
file yourself rather than pointing at a path. There is also no `fit` mode
(`cover`/`contain`) — the element you embed carries its own width/height. A
proper `image` node with path resolution, an asset directory and fit modes is
the next feature.

## Proof: the KORA landing page

`examples/kora.json` is a port of a 10-section ecommerce landing page that was
previously hand-authored as **4,052 px of absolute SVG coordinates** in Python
and rasterised with Inkscape. Same page, written as meaning:

| | KORA (original) | brailer |
|---|---|---|
| authoring | 457 lines, absolute `x/y` | 1 JSON spec, no coordinates |
| renderer | Inkscape subprocess | resvg, in-process |
| output | 1440×4052 | 1440×3307 (82%) |
| distinct colours | 4,337 | 556 |
| verify | none | `errors=0 collisions=0 PASS` |

The colour gap is honest: gradients, drop shadows and letter-spaced labels in
the original are not expressible in the spec vocabulary yet, and the port is
structurally faithful rather than pixel-identical. The point of the test is the
authoring model — a declarative spec reached a comparable full-page design
without a single hardcoded coordinate.

Renders in **984 ms** for SVG + 1× + 2× (layout 3 ms, SVG 1 ms).

## What it costs

**Money: nothing.** No API keys, no cloud service, no licence, no per-render
charge. Everything runs on your machine, offline. The dependencies are all
permissive-licensed crates (`resvg`/`usvg` are MPL-2.0-or-Apache, the rest MIT/
Apache-2.0).

**Disk:** 6.2 MB binary, linking only `libm`, `libgcc_s` and the C runtime — no
X11, no fontconfig, no browser.

**Time:** ~310 ms for a typical 1440×1087 page (1× + 2×), ~1 s for the full
1440×3307 KORA page. Layout and SVG generation are 1–3 ms; rasterisation is
effectively the whole cost.

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

Measured on a 1440×1087, 30-primitive spec: **verify 10 ms, render 312 ms**
(layout 1 ms, SVG 1 ms, raster 310 ms). A 12,801-primitive stress spec verifies
in **80 ms**.

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
- **Bespoke art direction is not automatable.** `raw` is an escape hatch, not a
  creative eye. Engine-owned layout, typography, colour and composition cover
  most of what makes a page work; the rest does not reduce to rules.
- **Only one theme ships** (`editorial`). More themes are the highest-leverage
  next feature.

## Licence

Not yet chosen — MIT and Apache-2.0 are the likely candidates.
