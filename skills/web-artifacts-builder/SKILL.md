---
name: web-artifacts-builder
description: Suite of tools for creating elaborate, multi-component HTML artifacts using modern frontend web technologies (React, Tailwind CSS, shadcn/ui). Use for complex artifacts requiring state management, routing, or shadcn/ui components - not for simple single-file HTML/JSX artifacts.
license: Complete terms in LICENSE.txt
---

# Web Artifacts Builder

To build powerful frontend web artifacts, follow these steps:

1. Initialize the frontend repo using `scripts/init-artifact.sh`
2. Develop your artifact by editing the generated code
3. Bundle all code into a single HTML file using `scripts/bundle-artifact.sh`
4. Publish the artifact and give the user its link
5. (Optional) Test the artifact

**Stack**: React 18 + TypeScript + Vite + Parcel (bundling) + Tailwind CSS + shadcn/ui

## When to use this skill

Use this skill when the artifact needs real application structure: multiple
components, client-side state, routing, or shadcn/ui primitives.

For a single page of content — a document, dashboard mockup, calculator, chart,
landing page, or anything you could hand-write in one HTML file — use the
**simple-artifact-builder** skill instead. It has no build step and finishes in
a fraction of the time.

## Design & Style Guidelines

Read `../STYLE_GUIDE.md` before writing any UI code and follow its tokens and
rules.

VERY IMPORTANT: To avoid what is often referred to as "AI slop", avoid using excessive centered layouts, purple gradients, uniform rounded corners, and Inter font.

## Quick Start

### Step 1: Initialize Project

Run the initialization script to create a new React project:

```bash
bash scripts/init-artifact.sh <project-name>
cd <project-name>
```

This creates a fully configured project with:

- ✅ React + TypeScript (via Vite)
- ✅ Tailwind CSS 3.4.1 with shadcn/ui theming system
- ✅ Path aliases (`@/`) configured
- ✅ 40+ shadcn/ui components pre-installed
- ✅ All Radix UI dependencies included
- ✅ Parcel configured for bundling (via .parcelrc)
- ✅ Node 18+ compatibility (auto-detects and pins Vite version)

### Step 2: Develop Your Artifact

To build the artifact, edit the generated files.

### Step 3: Bundle to Single HTML File

To bundle the React app into a single HTML artifact:

```bash
bash scripts/bundle-artifact.sh
```

This creates `bundle.html` - a self-contained artifact with all JavaScript, CSS, and dependencies inlined.

**Requirements**: Your project must have an `index.html` in the root directory.

**What the script does**:

- Installs bundling dependencies (parcel, @parcel/config-default, parcel-resolver-tspaths, html-inline)
- Creates `.parcelrc` config with path alias support
- Builds with Parcel (no source maps)
- Inlines all assets into single HTML using html-inline

Verify the result is genuinely self-contained before publishing — no `src="http`
or `href="http` references to scripts, stylesheets, fonts, or images:

```bash
grep -o -E '(src|href)="https?://[^"]*"' bundle.html | sort -u
```

### Step 4: Publish and Share with the User

Publish `bundle.html` using the **artifact-service** skill, then give the user
the `view_uri` it returns. That URL is the deliverable — a local file path is
not, since the user cannot open it in a browser from your working directory.

### Step 5: Testing/Visualizing the Artifact (Optional)

Note: This is a completely optional step. Only perform if necessary or requested.

To test/visualize the artifact, use available tools (including other Skills or built-in tools like Playwright or Puppeteer). In general, avoid testing the artifact upfront as it adds latency between the request and when the finished artifact can be seen. Test later, after presenting the artifact, if requested or if issues arise.

## Live updates

If the artifact will be updated in place, add this snippet after bundling so
viewers see new versions without refreshing manually. It reads the artifact UUID
from the URL and listens to `/api/artifacts/{id}/events`. Pinned historical
views (`?version=N`) ignore update events.

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
        '<main style="padding:2rem;font-family:system-ui,sans-serif"><h1>Deleted</h1><p>This artifact has been removed.</p></main>';
    });
  })();
</script>
```

## Reference

- **shadcn/ui components**: https://ui.shadcn.com/docs/components
- **Visual style**: `../STYLE_GUIDE.md`
- **Publishing**: `../artifact-service/SKILL.md`
