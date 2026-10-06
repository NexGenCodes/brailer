---
name: brailer
description: "Use when producing any visual design output — landing pages, hero sections, posters, product cards, pricing tables, social/blog headers, editorial layouts, or any request to 'design', 'mock up', 'create a poster/banner/flyer/deck cover', or export SVG/PNG. brailer is a deterministic design compiler: emit a semantic JSON spec, it resolves all geometry, typography and colour and rasterises SVG/PNG. Use instead of writing SVG markup or HTML/CSS by hand when a pixel-accurate static design is wanted."
metadata:
  author: brailer
  version: "0.1.0"
---

# brailer

A design compiler. You author **meaning** as JSON; the engine owns **all geometry**.

```
spec.json  ->  brailer  ->  design.svg + design@1x.png + design@2x.png
```

## The one rule

**Never emit coordinates, font sizes, hex colours or SVG markup directly.**

Declare *what a thing is* (`role: "h1"`, `gap`, `columns`), never *where it lands*.
The engine decides size, family, colour, leading and position — and it is right,
because it can measure text and you cannot see pixels.

The only escape hatch is `raw` (inline SVG) for bespoke artwork. Use it last, not first.

## Workflow

```
1. write  design.json
2. brailer verify design.json            # fix errors until exit 0
3. brailer render design.json --retina   # writes svg + @1x + @2x
```

`render` refuses to run if verification fails, so step 2 is not optional — it is the
same gate. Run it first to get readable errors before committing to a full render.

| command | does | gates on verify |
|---|---|---|
| `brailer verify <spec>` | report only, exit 0/1 | is the gate |
| `brailer build <spec>` | layout + write SVG, fast | no |
| `brailer render <spec> [--retina] [--scale N]` | SVG + PNGs | yes |
| `brailer themes` | list available themes | — |
| `brailer fonts` | show font discovery + registration | — |

Useful flags: `verify --json` for machine-readable output, `verify --strict` to make
contrast warnings fatal.

## Spec schema

```jsonc
{
  "canvas": { "width": 1440, "background": "#FAF7F2" },
  "theme": "editorial",
  "root": { "kind": "stack", "gap": 24, "pad": 48, "children": [ /* ... */ ] }
}
```

### Nodes

**`stack`** — vertical flow. `gap` (space between children), `pad` (inner inset),
`align` (`start`|`center`|`end`), `bg`, `children[]`.

**`grid`** — N equal columns. `columns` (1–32), `gap`, `align`, `pad`, `bg`,
`children[]`. Children are laid out row-major; items wrap to the next row.

**`text`** — a run of type. `text`, `role` (see below), `align`, `color`
(defaults to the role's colour), `max_measure` (line-length cap in px).

**`rule`** — a horizontal hairline. `thickness` (> 0), `color`. Snap-aligned to
the pixel grid so it renders crisp.

**`spacer`** — fixed vertical gap. `height` (>= 0).

**`card`** — a raised block. `bg`, `border`, `radius`, `pad`, `children[]`.

**`raw`** — inline SVG fragment. `svg`, `height`. Escape hatch only.

### Text roles

`display` (76px) · `h1` (48) · `h2` (32) · `h3` (21) · `body` (16) · `small` (13) · `label` (12)

Roles also pick family and colour for you:

| roles | family | colour |
|---|---|---|
| `display` `h1` `h2` | serif | `ink` |
| `h3` `body` `small` | sans | `ink` for `h3`, `muted` otherwise |
| `label` | sans | `accent` |

Trust this mapping. Override `color` only when the composition demands it.

### Themes

`brailer themes` lists them. `editorial` ships: warm paper background `#FAF7F2`,
near-black ink `#14110F`, terracotta accent `#A0492B`, hairline `#CEC2AE`.
All spacing derives from `space.base` (8px); radii are `sm`/`md`/`lg` (4/8/16).

## Verification report

```
prims=30 size=1440x1087 errors=0 out_of_bounds=0 collisions=0 contrast=0 -- PASS
```

- **errors** — invalid structure (bad colour, negative gap, empty text, unsafe SVG).
  Hard fail. Fix the spec.
- **out_of_bounds** — geometry ran off the canvas. Usually a `width` that is too
  large for the canvas, or content that needs more room. Reduce `max_measure`,
  shorten copy, or widen the canvas.
- **collisions** — two text runs overlap. Almost always copy too long for its slot.
  Shorten it or give the container more space.
- **contrast** — text/background below 4.5:1. Warning normally, fatal with `--strict`.

`--json` prints `ok`, `geometry_ok`, `prims`, `width`, `height` and the four arrays.

## Limits

| limit | value | why |
|---|---|---|
| canvas width | 320 – 8192 | sanity |
| nesting depth | 64 | stack safety |
| leaf nodes | 20,000 | parse + layout budget |
| columns | 1 – 32 | galleries/tables, not infinite |
| output dimension | 16,000 px | pixmap memory; lower `--scale` if you hit it |
| collision pairs | 20,000 | bounding verification cost |

Rejected outright: `<script>`, doctype/entity declarations, `javascript:` and
`data:text/html` URLs inside `raw`.

## Worked example

```json
{
  "canvas": { "width": 1440, "background": "#FAF7F2" },
  "theme": "editorial",
  "root": {
    "kind": "stack",
    "gap": 0,
    "pad": 0,
    "children": [
      { "kind": "stack", "gap": 8, "pad": 48, "bg": "#EFE7DB",
        "children": [
          { "kind": "text", "role": "label", "text": "NEW WORK" },
          { "kind": "text", "role": "h1", "text": "Solid ash, hand-planed" },
          { "kind": "text", "role": "body", "max_measure": 620,
            "text": "A seat three, without a central leg, finished in one workshop over thirty years of practice." }
        ] },
      { "kind": "rule", "thickness": 1 },
      { "kind": "stack", "gap": 24, "pad": 48, "children": [
        { "kind": "grid", "columns": 3, "gap": 24, "children": [
          { "kind": "card", "pad": 24, "radius": 8, "children": [
            { "kind": "text", "role": "h3", "text": "Oak" },
            { "kind": "text", "role": "small", "text": "Quarter-sawn, air-dried." }
          ] },
          { "kind": "card", "pad": 24, "radius": 8, "children": [
            { "kind": "text", "role": "h3", "text": "Brass" },
            { "kind": "text", "role": "small", "text": "Hand-finished hardware." }
          ] },
          { "kind": "card", "pad": 24, "radius": 8, "children": [
            { "kind": "text", "role": "h3", "text": "Linen" },
            { "kind": "text", "role": "small", "text": "Undyed, washed finish." }
          ] }
        ] }
      ] }
    ]
  }
}
```

## Designing well

- **One idea per band.** Stack flat sections separated by `rule`, not deep nesting.
- **Let measure do the work.** `max_measure` on body copy (~620px) is the single
  biggest quality lever — long lines are what make AI layouts look amateur.
- **Rhythm from the theme.** Use `gap` values that are multiples of the 8px base.
  `pad: 48`, `gap: 24`, `gap: 64` read as deliberate; `gap: 17` does not.
- **Contrast comes from roles, not hex.** Prefer changing `role` over `color`.
- **Hierarchy is scale, not weight.** Let `h1` → `body` do the ordering.
- **You cannot see the output.** Trust `verify` for structure and geometry; state
  plainly that aesthetic review needs a human. Do not claim a design looks good.

## Known limits

Layout, typography, colour and composition are engine-owned and verifiable.
Bespoke art direction is not automatable — a `raw` escape hatch is not a
creative eye. Verification is structural, geometric and pixel-statistical; it
cannot judge taste.
