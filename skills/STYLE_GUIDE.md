# Artifact Style Guide

A shared visual language for artifacts built with the `simple-artifact-builder`
and `web-artifacts-builder` skills. The target aesthetic is **Bauhaus-inspired
and phone-first**: paper and ink, bold geometric type, saturated primary
accents, and composition built for a 375px column before anything wider.

Follow this guide unless the user asks for something specific. If they do, their
request wins.

## Principles

1. **Form follows function.** Nothing decorates that does not structure. If a
   shape, rule, or color does not organize the page, remove it.
2. **Paper and ink first.** A warm off-white ground, near-black ink, and type
   carry the design. Primary color is earned, not sprayed.
3. **Pure geometry.** Corners are sharp (`0`) or perfectly round (`9999px`) —
   nothing in between. Squares, circles, and triangles are the only motifs.
4. **One primary per page.** Red, blue, and yellow each have a job. One carries
   the page's action; the others appear only as small marks, if at all.
5. **Phone first.** Style the single 375px column, then use `min-width` queries
   to add complexity upward. Every touch target clears 44px.

## Anti-patterns

These read as generic AI output or break the system. Avoid them:

- Purple/violet gradient backgrounds or gradient headline text
- Any gradient — the system is flat color on flat color
- Every element with a soft default `border-radius` (the "pill soup" look)
- Inter, or any font specified without a fallback stack
- Everything centered, including body copy and cards
- Emoji as section icons or bullets
- Drop shadows — depth comes from borders and value, never blur
- Three-column "feature card" grids with an icon, bold word, and one grey line
- Pastel or desaturated palettes — primaries stay saturated
- Yellow as a text color — yellow is a background with ink text on it
- Faux-Bauhaus clutter: geometric shapes crammed into every section. One motif
  per page.

## Tokens

Paste this into your `<style>` block and build on top of it.

```css
:root {
  /* Paper — layered from ground up */
  --paper: #f4f1e8;
  --paper-raised: #fbf9f3;

  /* Ink — three levels, no more */
  --ink: #1a1a1a;
  --ink-muted: #57544a;
  --ink-faint: #8a877b;

  /* Primaries — saturated, one role each */
  --red: #c2332b;      /* the page's action */
  --red-ink: #9e2a23;  /* hover/darker step */
  --blue: #1f4fb8;     /* links, focus */
  --yellow: #e8a80c;   /* background blocks only, ink text on top */

  /* Rules */
  --rule: #1a1a1a;       /* 2px structural rules and borders */
  --rule-soft: #d8d4c6;  /* hairline row separators */

  --font-sans:
    "Futura", "Avenir Next", "Century Gothic", "Segoe UI", system-ui,
    Roboto, "Helvetica Neue", Arial, sans-serif;

  /* 4px scale */
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-3: 0.75rem;
  --space-4: 1rem;
  --space-6: 1.5rem;
  --space-8: 2rem;
  --space-12: 3rem;
  --space-16: 4rem;

  --radius: 0;
  --radius-circle: 9999px;
}
```

Notes:

- `--ink-faint` is for decorative text only (placeholders, watermarks) — never
  for anything a reader must parse.
- `--yellow` never appears as text and never on a light ground with light text.
  It is a filled block with `--ink` text.
- Numbers use `font-variant-numeric: tabular-nums` on the sans stack — no
  monospace anywhere in the design voice.

### Dark mode

Optional. If you support it, swap only the tokens — never restate colors
inline. Inverted paper, same primaries brightened for contrast:

```css
@media (prefers-color-scheme: dark) {
  :root {
    --paper: #161511;
    --paper-raised: #1e1d17;
    --ink: #f2efe6;
    --ink-muted: #b5b1a4;
    --ink-faint: #7d7a6e;
    --red: #e0524a;
    --red-ink: #f07770;
    --blue: #7d9de8;
    --yellow: #f2c14e;
    --rule: #f2efe6;
    --rule-soft: #3a3931;
  }
}
```

## Typography

| Role            | Font | Size                        | Weight | Notes                                       |
| --------------- | ---- | --------------------------- | ------ | ------------------------------------------- |
| Display title   | sans | `clamp(2rem, 7vw, 3.25rem)` | 700    | `uppercase`, `line-height: 1.05`            |
| Section heading | sans | `clamp(1.25rem, 3vw, 1.75rem)` | 700 | `uppercase`, generous space above, tight below |
| Body            | sans | 1rem                        | 400    | `line-height: 1.6`, `--ink`                 |
| Secondary body  | sans | 0.9375rem                   | 400    | `--ink-muted`                               |
| Label           | sans | 0.8125rem                   | 700    | `uppercase`, `letter-spacing: 0.08em`       |
| Numbers         | sans | inherit                     | 600    | `font-variant-numeric: tabular-nums`, right-aligned |

Hierarchy comes from weight, case, size, and space — headings never get their
own color. Uppercase is reserved for display type and labels; body copy stays
sentence case.

The one exception to the sans rule: if the page shows **code**, use the system
mono stack for that content alone (`ui-monospace, Menlo, Consolas, monospace`).
That is a functional requirement, not part of the design voice — never use mono
for labels, eyebrows, or tables.

## Layout

- Base styles are the phone column: one column, no grid, no fixed widths.
- Page container: `max-width: 68rem`, `margin-inline: auto`, modest padding
  (`var(--space-6)` top, `var(--space-4)` inline) that grows at `48rem`.
- Prose blocks: cap at ~35rem — long measures are unreadable on phones anyway.
- Vertical rhythm: `--space-12` between major sections, `--space-6` inside
  them. Consistency matters more than the exact values.
- Grids only exist at `@media (min-width: 48rem)`:
  `repeat(auto-fit, minmax(16rem, 1fr))`, `gap: var(--space-4)`. Prefer two
  columns of substance over four of filler.

## Built for phones first

- **Fluid type**: `clamp()` on display sizes so headlines never overflow a
  320px viewport and never balloon on desktop.
- **Touch targets**: buttons and tappable controls are at least `min-height:
  44px` with real padding — the iOS/Android minimum, comfortably met.
- **Input font-size ≥ 16px**: prevents iOS Safari from auto-zooming on focus.
- **Hover is optional**: wrap every `:hover` rule in
  `@media (hover: hover)` so touch devices never get sticky hover states, and
  never hide information behind hover only.
- **Overflow is contained**: any wide element (table, code block, diagram)
  scrolls inside its own `overflow-x: auto` container — the page body must
  never scroll horizontally.
- **Safe areas**: honor `env(safe-area-inset-*)` in page padding when content
  runs to the edges.
- Motion is limited to 120–200ms transitions on `border-color`, `background`,
  and `opacity`. No entrance animations, parallax, or autoplaying motion.
  Honor `prefers-reduced-motion: reduce` by disabling transitions.

## Components

**Cards** — `background: var(--paper-raised)`, `border: 2px solid var(--rule)`,
`border-radius: var(--radius)`, `padding: var(--space-6)`. No shadow, no hover
state unless the card is itself a link or button.

**Buttons** — flat blocks, `text-transform: uppercase`, weight 700,
`border-radius: var(--radius)`, `padding: var(--space-3) var(--space-5)`,
`min-height: 44px`. Primary: `background: var(--red)`, color `var(--paper)`;
hover `var(--red-ink)`. Secondary: transparent with `2px solid var(--rule)` and
`--ink`; hover fills `--ink`/`--paper`. Always give a visible `:focus-visible`
outline in `--blue`, `2px`, offset 2px.

**Links** — `color: var(--blue)`, underlined in body copy. Inline text links
get `padding: var(--space-2) 0` or equivalent so their tap area reaches 44px.

**Tables** — header row in label style (uppercase, bold, small) with a
`2px solid var(--rule)` bottom rule. Row separators are hairlines
(`--rule-soft`) only, never a full grid. Right-align numbers with
`font-variant-numeric: tabular-nums`. On phones the table scrolls inside an
`overflow-x: auto` wrapper; hide non-essential columns under `40rem`.

**Geometric marks** — the system's only decoration. Small filled squares,
circles, and triangles (inline SVG or CSS) used as bullets, section markers, or
one header motif. Saturated primaries on ink, or ink on primaries. One motif
per page.

**Code blocks** — if the page needs them: `background: var(--paper-raised)`,
`border: 1px solid var(--rule-soft)`, mono stack for the code only,
`padding: var(--space-4)`, `overflow-x: auto`. Never wrap code.

## Accessibility

Body text must clear 4.5:1 contrast against its background (the tokens above
do; `--ink-faint` is decorative only). Never signal state by color alone —
pair it with text or a shape. Use real semantic elements (`<button>`,
`<nav>`, `<table>`, one `<h1>` per page) so keyboard and screen-reader
navigation works.

## Starter shell

```html
<body>
  <main class="page">
    <header class="page-header">
      <div class="motif" aria-hidden="true">
        <span class="motif-square"></span>
        <span class="motif-circle"></span>
        <span class="motif-triangle"></span>
      </div>
      <h1>Artifact title</h1>
      <p class="lede">One sentence on what this page shows.</p>
    </header>
    <section><!-- content --></section>
  </main>
</body>
```

```css
* {
  box-sizing: border-box;
}
body {
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: var(--font-sans);
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}
.page {
  max-width: 68rem;
  margin-inline: auto;
  padding: var(--space-8) var(--space-4);
}
@media (min-width: 48rem) {
  .page {
    padding: var(--space-12) var(--space-6);
  }
}
.page-header {
  margin-bottom: var(--space-12);
}
.motif {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  margin-bottom: var(--space-4);
}
.motif-square {
  width: 0.875rem;
  height: 0.875rem;
  background: var(--ink);
}
.motif-circle {
  width: 0.875rem;
  height: 0.875rem;
  border-radius: var(--radius-circle);
  background: var(--red);
}
.motif-triangle {
  width: 0;
  height: 0;
  border-left: 0.5rem solid transparent;
  border-right: 0.5rem solid transparent;
  border-bottom: 0.875rem solid var(--blue);
}
h1 {
  margin: 0;
  font-size: clamp(2rem, 7vw, 3.25rem);
  font-weight: 700;
  line-height: 1.05;
  text-transform: uppercase;
}
.lede {
  max-width: 35rem;
  margin-top: var(--space-4);
  font-size: 1.0625rem;
  color: var(--ink-muted);
}
```
