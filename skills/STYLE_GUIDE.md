# Artifact Style Guide

A shared visual language for artifacts built with the `simple-artifact-builder`
and `web-artifacts-builder` skills. The target aesthetic is a **modern developer
terminal**: dark, quiet, text-forward, with monospace used as a deliberate
accent rather than decoration.

Follow this guide unless the user asks for something specific. If they do, their
request wins.

## Principles

1. **Dark by default.** Near-black ground, off-white text, one accent color.
2. **Text is the interface.** Typography and spacing carry the design; borders
   and color do the minimum needed to separate things.
3. **Restraint over ornament.** No decoration that does not clarify structure.
4. **Left-aligned and readable.** Content sits on a consistent left edge with a
   measure of 65–75 characters. Center only isolated hero text.

## Anti-patterns

These read as generic AI output. Avoid them:

- Purple/violet gradient backgrounds or gradient headline text
- Every element with the same large `border-radius` (the "pill soup" look)
- Inter, or any font specified without a fallback stack
- Everything centered, including body copy and cards
- Emoji as section icons or bullets
- Drop shadows on flat dark surfaces (they do nothing; use borders instead)
- Three-column "feature card" grids with an icon, bold word, and one grey line

## Tokens

Paste this into your `<style>` block and build on top of it.

```css
:root {
  /* Surfaces — layered from ground up */
  --bg: #0a0a0a;
  --surface: #121212;
  --surface-raised: #181818;
  --border: #262626;
  --border-strong: #3a3a3a;

  /* Text — three levels, no more */
  --text: #ededed;
  --text-muted: #a1a1a1;
  --text-faint: #6e6e6e;

  /* One accent, used sparingly */
  --accent: #7dd3a8;
  --accent-dim: #2f5c47;

  /* Status */
  --danger: #f87171;
  --warning: #fbbf24;

  --font-sans:
    ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto,
    "Helvetica Neue", Arial, sans-serif;
  --font-mono:
    ui-monospace, "SF Mono", "JetBrains Mono", "Fira Code", Menlo, Consolas,
    monospace;

  /* 4px scale */
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-3: 0.75rem;
  --space-4: 1rem;
  --space-6: 1.5rem;
  --space-8: 2rem;
  --space-12: 3rem;
  --space-16: 4rem;

  --radius: 4px;
  --radius-lg: 8px;
  --measure: 68ch;
}
```

### Light mode

Optional. If you support it, swap only the tokens — never restate colors inline:

```css
@media (prefers-color-scheme: light) {
  :root {
    --bg: #fafafa;
    --surface: #ffffff;
    --surface-raised: #f4f4f4;
    --border: #e4e4e4;
    --border-strong: #d4d4d4;
    --text: #171717;
    --text-muted: #525252;
    --text-faint: #8a8a8a;
    --accent: #1f7a56;
    --accent-dim: #b8e2cd;
  }
}
```

## Typography

| Role                | Font | Size        | Weight | Notes                                 |
| ------------------- | ---- | ----------- | ------ | ------------------------------------- |
| Page title          | sans | 2–2.5rem    | 600    | `letter-spacing: -0.02em`             |
| Section heading     | sans | 1.25–1.5rem | 600    | Generous space above, tight below     |
| Body                | sans | 1rem        | 400    | `line-height: 1.65`, `--text`         |
| Secondary body      | sans | 0.9375rem   | 400    | `--text-muted`                        |
| Eyebrow / label     | mono | 0.75rem     | 500    | `uppercase`, `letter-spacing: 0.08em` |
| Code, data, metrics | mono | 0.875rem    | 400    | Numbers in tables always monospace    |

Headings never get their own color or gradient — hierarchy comes from size,
weight, and spacing. Monospace signals "machine-generated": labels, IDs,
timestamps, code, and numeric columns. Prose stays sans-serif.

## Layout

- Page container: `max-width: 72rem`, `margin-inline: auto`, `padding: var(--space-8) var(--space-6)`.
- Prose blocks: cap at `var(--measure)`.
- Vertical rhythm: `--space-16` between major sections, `--space-6` inside them.
  Consistency matters more than the exact values.
- Grids: `repeat(auto-fit, minmax(16rem, 1fr))` with `gap: var(--space-4)`.
  Prefer two columns of substance over four of filler.

## Components

**Cards** — `background: var(--surface)`, `border: 1px solid var(--border)`,
`border-radius: var(--radius-lg)`, `padding: var(--space-6)`. No shadow. On
hover (only if interactive): `border-color: var(--border-strong)`.

**Buttons** — Primary: `background: var(--accent)`, `color: var(--bg)`,
`font-weight: 500`, `border-radius: var(--radius)`, `padding: var(--space-2) var(--space-4)`.
Secondary: transparent with `1px solid var(--border-strong)` and `--text`.
Always give a visible `:focus-visible` outline in `--accent`.

**Tables** — Header row in mono eyebrow style with a `--border` bottom rule.
Row separators only, never a full grid. Right-align and monospace numbers.

**Code blocks** — `background: var(--surface)`, `border: 1px solid var(--border)`,
`padding: var(--space-4)`, `overflow-x: auto`. Never wrap code.

**Terminal accents** — Used sparingly, one or two per page at most: a `$ ` or
`>` prefix on a command line, an ASCII rule (`────────`) between major sections,
or a blinking block cursor after a hero heading. Skip them entirely on
data-heavy pages.

## Responsiveness and motion

Use fluid widths, `max-width: 100%` on media, and collapse multi-column grids to
one column under ~40rem. Any wide element (table, code block, diagram) scrolls
inside its own `overflow-x: auto` container — the page body must never scroll
horizontally.

Motion is limited to 120–200ms transitions on `border-color`, `background`,
`opacity`, and `transform`. No entrance animations, parallax, or autoplaying
motion. Honor `prefers-reduced-motion: reduce` by disabling transitions.

## Accessibility

Body text must clear 4.5:1 contrast against its background (the tokens above
do). Never signal state by color alone — pair it with text or an icon. Use real
semantic elements (`<button>`, `<nav>`, `<table>`, one `<h1>` per page) so
keyboard and screen-reader navigation works.

## Starter shell

```html
<body>
  <main class="page">
    <header class="page-header">
      <p class="eyebrow">Section label</p>
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
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-sans);
  line-height: 1.65;
  -webkit-font-smoothing: antialiased;
}
.page {
  max-width: 72rem;
  margin-inline: auto;
  padding: var(--space-8) var(--space-6);
}
.page-header {
  margin-bottom: var(--space-12);
}
.eyebrow {
  margin: 0 0 var(--space-2);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--accent);
}
h1 {
  margin: 0;
  font-size: 2.25rem;
  font-weight: 600;
  letter-spacing: -0.02em;
}
.lede {
  max-width: var(--measure);
  margin-top: var(--space-4);
  color: var(--text-muted);
}
```
