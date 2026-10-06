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
