---
name: brailer
description: "Use when producing any visual design output — app screens (auth/login/signup, home feed, marketplace/store home, cart/checkout, settings), landing pages, hero sections, product cards, pricing tables, banners/posters, editorial layouts, dashboards, or any request to 'design', 'mock up', 'create a screen/poster/flyer', or export SVG/PNG. brailer is a deterministic design compiler: you author a semantic JSON spec for desktop and mobile frames, it owns every coordinate, px font size, hex colour, line break and overlap check, and rasterises SVG/PNG. Use it instead of hand-writing SVG or HTML/CSS when a pixel-accurate static design is wanted — this tool is how the AI models itself as a UI/UX designer with measurable guarantees."
metadata:
  author: brailer
  version: "0.2.0"
---

# brailer

A design compiler — and the engine behind your UI/UX designer hat. You author
**meaning** as JSON; the engine owns **all geometry** — and proves it with a
verify gate before a single pixel is drawn.

```
spec.json  ->  brailer  ->  <frame>.svg + <frame>@<scale>.png   (one per frame)
```

## The one rule

**Never emit coordinates, font sizes, hex colours or SVG markup directly.**

Declare *what a thing is* (`role: "h1"`, `gap`, `columns`, `span`), never
*where it lands*. The engine decides size, family, colour, leading, wrapping
and position — and it is right, because it measures text and you cannot see
pixels.

The only escape hatch is `raw` (inline SVG) for bespoke artwork. Use it last.

## Act as a designer (your job)

The engine handles craft; **your** job is the design brief. Before writing any
JSON, decide each of these — then the spec writes itself:

1. **Goal.** What is this one screen for? (Auth = trust + one path. Store home =
   reduce browsing friction. Checkout = no exits.) One screen, one job.
2. **Layout for two canvases.** Real apps are responsive. Ship a `desktop`
   frame (1440) *and* a `mobile` frame (390) for every screen — never one
   canvas. Same content, different composition.
3. **Hierarchy by role.** One idea per band. `display`/`h1` once per screen.
   `h2` for sections. `h3` for cards. `body` for prose, `small` for scannable
   meta, `label` (uppercase, tracked) for eyebrows and tags. Hierarchy is scale,
   not weight.
4. **Rhythm from the 8px base.** `gap`/`pad` in multiples of 8. `16/24/32/40/48`
   read as deliberate; `17` reads as a bug.
5. **Colour with intent (canary theme):** paper `#FBF9F3` backdrop, surface
   `#FFFFFF` cards, graphite `#17161A` type, muted `#655F55` secondary, band-red
   `#B3261E` for price/CTAs/errors, canary `#F7D24B` for highlights/badges,
   hairline `#CFC7B4`. Red screams action — use it sparingly. Trust a dark band
   (ink → a step darker) for footer/hero weight.
6. **Ask the human for the aesthetic call** when it matters. You shape taste;
   you cannot confirm it on pixels alone.

### Section design playbook

| screen | band order that converts/reads well (desktop) |
|---|---|
| auth | brand → one headline → one supporting line → form card → trust line |
| store/home | promo strip → nav → category chips → hero (copy + cover image) → deals band → product grid (spans for a featured card) → spines/grades → social proof → newsletter → footer |
| checkout | nav → [form (weighted 5 cols) \| order card (3 cols)] → guarantee line |
| landing | eyebrow label → headline → subcopy → CTA → feature grid → proof → footer |

Product grids: 4 columns desktop, 2 columns mobile. Make one cell a
`cell { "span": 2 }` (or 4 = hero) so the featured item breaks the monotony.

### Accessibility you must design in

- `max_measure` ≤ ~560px on body copy; long lines are the #1 amateur tell.
- Keep CTAs ≥ ~40px tall (pad a `card`); never nest text below 11px.
- Never put `ink` text on `ink` fills — on dark bands use `color: "#FBF9F3"`.
- The gate enforces 4.5:1 contrast and zero collisions; let it, don't fight it.

## Workbook

### Responsive frames

```jsonc
{
  "theme": "canary",
  "frames": {
    "desktop": { "canvas": { "width": 1440, "background": "#FBF9F3" },
                 "root": { "kind": "stack", "pad": 40, "gap": 24, "children": [ /* ... */ ] } },
    "mobile":  { "canvas": { "width": 390,  "background": "#FBF9F3" },
                 "root": { "kind": "stack", "pad": 20, "gap": 16, "children": [ /* ... */ ] } }
  }
}
```

A legacy `canvas` + `root` spec still works and becomes a single `design` frame.

- Mobile: pad 16–20, gap 16–20, single-column stacks, grids collapse to 2 cols.
- Desktop: pad 40, gap 24–40, grids 3–12 cols, `cell` spans for asymmetric rows.
- Reuse shared pieces as **components** (nav, product card, footer) so the two
  frames cannot drift apart.

### Components

```jsonc
"components": {
  "brand": { "kind": "stack", "gap": 2, "children": [
    { "kind": "text", "role": "h2", "text": "CEDAR & LEAD", "tracking": 0.05 },
    { "kind": "text", "role": "label", "text": "graphite · wax · tools" } ] }
},
"root": { "kind": "stack", "children": [ { "kind": "component", "name": "brand" } ] }
```

`component` refs are inlined at load time with cycle detection, so any validator
already sees a fully-expanded tree.

## Image (first-class)

```jsonc
{ "kind": "image", "src": "photo.jpg", "width": 420, "height": 260, "fit": "cover" }
```

- `src` is either a `data:` URI, or a path resolved at load time against
  `asset_dir` (relative to the spec's folder). Engine emits `data:` URIs into
  the SVG and rasterises them.
- `fit`: `cover` (crop to fill — product photos, heroes), `contain` (letterbox),
  `fill` (stretch). Default `cover`.
- In SVG, the matching `preserveAspectRatio` (`slice`/`meet`/none) lands and the
  element is emitted as `<image>`, so photos are true first-class content,
  not `raw` escapes.

## Gradients and shadows

- `bg` on `stack`/`grid`/`card` can be a colour **or** a two-stop linear gradient:
  `{ "from": "#17161A", "to": "#201F25", "angle": 90 }` (angle = direction in
  degrees, `90` = top→bottom). Rendered as `<linearGradient>` defs.
- `shadow: { "blur": 14, "x": 0, "y": 3, "color": "#17161A1F" }` on
  `stack`/`grid`/`card` emits a named `<feDropShadow>` filter. Use soft, brown-ish
  shadows under cards, not outlines.

## Letter-spacing

`tracking` on `text` is in *em*, e.g. `0.04` on a display headline or `0.14` on
uppercase labels. Modest values (0.02–0.08) — enormous values are what make AI
letter-spacing look like a bug.

## Grid spanning

```jsonc
{ "kind": "grid", "columns": 12, "children": [
  { "kind": "cell", "span": 8,  "child": { /* hero */ } },
  { "kind": "cell", "span": 4,  "child": { /* sidebar card */ } } ] }
```

A cell spanning multiple columns makes one grid row asymmetric; cells that
don't fit a row wrap to the next and later cells pack beside the span — the
float-grid pack behind real marketplaces. `cell` is only valid as a direct
child of `grid`.

## Nodes

`stack` (vertical flow: `gap`, `pad`, `align`, `bg`, `shadow`) · `grid`
(`columns` 1–32, `gap`, `pad`, `bg`, `shadow`) · `cell` (`span`, `child`) ·
`text` (`role`, `text`, `align`, `color`, `max_measure`, `tracking`) · `image`
(`src`, `width?`, `height`, `fit`) · `rule` (`thickness`>0, `color`) ·
`spacer` (`height`≥0) · `card` (`bg`, `border`, `radius`, `pad`, `shadow`) ·
`raw` (escape hatch: `svg`, `height`) · `component` (`name`).

`text` roles (sizes shown for `canary`): `display` 64 · `h1` 40 · `h2` 28 ·
`h3` 19 · `body` 15 · `small` 13 · `label` 11.

Role → family/colour mapping: `display` `h1` `h2` serif/ink; `h3` sans/ink;
`body` `small` sans/muted; `label` sans/accent. Trust it; override `color` only
when the composition demands (e.g. light text on dark bands).

## Workflow

```
1. write  design.json          # desktop + mobile frames
2. brailer verify design.json  # fix until exit 0 (every frame)
3. brailer render design.json --retina   # <frame>.svg + <frame>@1x.png + @2x.png
4. brailer render design.json --frame mobile --retina   # just one frame
```

`render` refuses to run if verification fails anywhere. Every requested scale is
pre-flighted before any file is written — you are never told "PASS" and then
given a file error.

| command | does | gates on verify |
|---|---|---|
| `brailer verify <spec> [--frame NAME] [--json] [--strict]` | report, exit 0/1 | is the gate |
| `brailer build <spec>` | layout + write SVG, fast | no |
| `brailer render <spec> [-o out] [--retina] [--scale N] [--frame NAME]` | SVG + PNGs | yes |
| `brailer themes` | list themes | — |
| `brailer fonts` | font discovery + registration | — |

`--strict` makes contrast warnings fatal. `--json` emits the same numbers
machine-readable.

## Verification report

```
[desktop] prims=168 size=1440x4047 errors=0 out_of_bounds=0 collisions=0 contrast=0 -- PASS
[mobile]  prims=171 size=390x5296  errors=0 out_of_bounds=0 collisions=0 contrast=0 -- PASS
```

- **errors** — invalid structure (bad colour, negative gap, empty text, unsafe
  SVG, unresolved image, unknown component, cell outside a grid). Hard fail.
- **out_of_bounds** — geometry ran off a canvas. Shorten copy, lower
  `max_measure`, or widen the frame.
- **collisions** — two text runs overlap. Almost always copy too long for its
  slot. Shorten it or give the container more room.
- **contrast** — text/fill below 4.5:1. Warn normally, fatal with `--strict`.

## Limits

| limit | value |
|---|---|
| canvas width | 320 – 8192 |
| nesting depth | 64 |
| leaf nodes | 20,000 |
| columns | 1 – 32 |
| output dimension | 16,000 px (lower `--scale` if you hit it) |
| collision pairs | 20,000 |

Rejected outright in `raw`: `<script>`, doctype/entity declarations,
`javascript:` and `data:text/html` URLs.

## Worked example — a responsive band

```jsonc
{ "theme": "canary",
  "frames": {
    "desktop": { "canvas": { "width": 1440, "background": "#FBF9F3" }, "root": {
      "kind": "stack", "gap": 40, "pad": 40, "children": [
        { "kind": "grid", "columns": 12, "gap": 24, "children": [
          { "kind": "cell", "span": 8, "child": { "kind": "stack", "gap": 16, "pad": 40,
            "bg": { "from": "#17161A", "to": "#3A3941", "angle": 135 },
            "children": [
              { "kind": "text", "role": "label", "text": "NEW WORK", "color": "#F7D24B", "tracking": 0.14 },
              { "kind": "text", "role": "h1", "text": "Pencils worth keeping.", "color": "#FBF9F3", "max_measure": 520 },
              { "kind": "text", "role": "body", "text": "Every sharpen ships with a cedar pointer.", "color": "#FBF9F3" } ] } },
          { "kind": "cell", "span": 4, "child": { "kind": "card", "radius": 16, "pad": 24,
            "shadow": { "blur": 14, "x": 0, "y": 3 }, "children": [
              { "kind": "text", "role": "h3", "text": "The kit" },
              { "kind": "text", "role": "small", "text": "HB · 2B · 4H, boxed." } ] } } ] }
      ] } } } }
```

## Designing well — checklist

- [ ] Two frames? Desktop **and** mobile, same content, mobile-specific flow?
- [ ] One `h1` per screen; every section labelled with an eyebrow `label`?
- [ ] Body copy capped with `max_measure`?
- [ ] Rhythm: all `gap`/`pad` multiples of 8?
- [ ] Red reserved for action; dark band carries the footer/hero weight?
- [ ] Product/photos via `image` + `fit: cover`, not `raw`?
- [ ] One `cell` span in every grid that needs a featured item?
- [ ] Contrast: light text on every dark fill?

## Known limits

Layout, typography, colour and composition are engine-owned and verifiable.
Bespoke art direction is not automatable — `raw` is an escape hatch, not a
creative eye. Verification is structural, geometric and pixel-statistical; it
cannot judge taste. Two themes ship; more themes are the highest-leverage
extension. State plainly when aesthetic review needs a human — do not claim a
design "looks good" from geometry numbers alone.