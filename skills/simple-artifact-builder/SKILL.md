---
name: simple-artifact-builder
description: Build a self-contained single-file HTML artifact with no build step - inline CSS and JS, no external requests. Use for documents, dashboards, calculators, charts, landing pages, and other single-page artifacts. For multi-component apps needing state management, routing, or shadcn/ui, use web-artifacts-builder instead.
---

# Simple Artifact Builder

Produces one `index.html` that opens correctly with no server, no network, and
no build step. This is the fast path — reach for it first, and escalate to
`web-artifacts-builder` only when the page genuinely needs an application
framework.

## When to use this skill

**Use this skill for:** reports and documents, dashboards, data tables, charts,
calculators, forms, timelines, landing pages, diagrams, reference cards,
interactive explainers — anything that is one page of content, even if it has
some interactivity.

**Use `web-artifacts-builder` instead when:** the artifact needs client-side
routing across multiple views, shared state across many components, or
shadcn/ui components specifically. When you are torn, start here — a single file
you finish in one pass beats a React scaffold you abandon halfway.

## Hard requirements

The output must be **one file that works offline**:

- Everything inline: one `<style>` block, one `<script>` block. No external
  `.css` or `.js` files.
- **No CDN links, no remote fonts, no remote images, no `fetch()` to any host.**
  Use system font stacks (see the style guide) and inline SVG for graphics. If a
  raster image is unavoidable, embed it as a `data:` URI.
- No frameworks. Vanilla JS handles everything at this scale. If you find
  yourself hand-rolling a component system, switch skills.
- `<!doctype html>`, `<html lang="...">`, `<meta charset="utf-8">`, a viewport
  meta tag, and a meaningful `<title>` — the title names the artifact in browser
  tabs.

## Workflow

1. **Read `../STYLE_GUIDE.md`** and copy its `:root` token block as your
   starting CSS. Do this before writing markup.
2. **Write the content first.** Semantic HTML: `<main>`, `<section>`,
   `<header>`, `<table>`, one `<h1>`. Real content, not lorem ipsum.
3. **Style with the tokens.** Reference `var(--…)` rather than hard-coded hex
   values, so the palette stays consistent and swappable.
4. **Add behavior last, only where it earns its place.** Attach listeners with
   `addEventListener`, not inline `onclick`.
5. **Verify** (below), then **publish** with the `artifact-service` skill and
   give the user the returned `view_uri`.

## Structure

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Quarterly Metrics</title>
    <style>
      /* :root tokens from STYLE_GUIDE.md, then page styles */
    </style>
  </head>
  <body>
    <main class="page">
      <!-- content -->
    </main>
    <script>
      // behavior, if any
    </script>
  </body>
</html>
```

## Charts and diagrams

Draw them yourself with inline SVG — it is sharp at any size, styleable with the
same CSS tokens, and adds no dependency. Compute coordinates in the `<script>`
when the data is dynamic, or hard-code the path when it is fixed. Do not pull in
a charting library from a CDN; that breaks the offline requirement.

Give every chart an accessible fallback: a `<title>` element inside the `<svg>`,
or a visually adjacent table of the same numbers.

## Verify before publishing

```bash
# Must print nothing — any hit is an external dependency that will break offline.
grep -o -E '(src|href)="https?://[^"]*"' index.html
grep -n -E 'cdn\.|unpkg|jsdelivr|fonts\.googleapis' index.html
```

Then open the file in a browser (or read it back) and confirm: it renders with
no console errors, the layout holds at both ~375px and ~1440px wide, and no
horizontal scrollbar appears on the body.

## Common mistakes

- Pulling Tailwind, Chart.js, or a font from a CDN "just this once" — the
  artifact then renders broken for anyone offline or behind a strict network.
- Emitting a fragment instead of a full document; the file must start with
  `<!doctype html>`.
- Leaving the default `<title>Document</title>`.
- Fixed pixel widths that overflow on a phone.
- Handing the user a local file path instead of publishing and returning the
  `view_uri`.

## Live updates

If the artifact will be updated in place, include this snippet so viewers see
changes without refreshing manually. It reads the artifact UUID from the URL and
listens to `/api/artifacts/{id}/events`. Pinned historical views (`?version=N`)
ignore update events and stay on the selected version.

```html
<script>
  "use strict";
  (function () {
    const match = location.pathname.match(/^\/a\/([0-9a-f-]{36})/);
    if (!match) return;
    const source = new EventSource("/api/artifacts/" + match[1] + "/events");
    source.addEventListener("update", function () {
      if (!location.search.includes("version=")) {
        location.reload();
      }
    });
    source.addEventListener("deleted", function () {
      document.body.innerHTML =
        '<main class="page"><h1>Deleted</h1><p>This artifact has been removed.</p></main>';
    });
  })();
</script>
```

## Reference

- **Visual style**: `../STYLE_GUIDE.md`
- **Publishing**: `../artifact-service/SKILL.md`
