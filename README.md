# artifacts

A small self-hosted service for publishing self-contained HTML pages, each at a
stable public URL — a local equivalent of hosted "artifacts".

Upload one `index.html`, get back a link. Update it later and the link keeps
working, now serving the new version, with every previous version still
addressable.

Ships with three LLM skills (in `skills/`) so an agent can build an artifact and
publish it without further instruction.

## Quick start

```bash
docker compose up --build -d

curl -sS -X POST -H 'Content-Type: text/html' \
  --data-binary @page.html \
  "http://localhost:8080/api/artifacts?title=My%20Page"
```

The response contains a `view_uri` — open it in a browser.

Open the service root (`http://localhost:8080/`) for the dashboard: every
artifact in one table, with its version history, a copy button for its shareable
link, and delete.

To run without Docker: `cargo run` (listens on `:8080`, stores under `./data`).

## Configuration

All configuration is via environment variables.

| Variable          | Default                 | Purpose                                            |
| ----------------- | ----------------------- | -------------------------------------------------- |
| `BIND_ADDR`       | `0.0.0.0:8080`          | Listen address. Takes precedence over `PORT`.      |
| `PORT`            | `8080`                  | Port, when `BIND_ADDR` is unset.                   |
| `DATA_DIR`        | `./data`                | Where artifacts are stored (`/data` in the image). |
| `PUBLIC_BASE_URL` | `http://localhost:8080` | Base URL used to build `view_uri`.                 |
| `MAX_BODY_BYTES`  | `10485760` (10 MiB)     | Maximum artifact size.                             |
| `RUST_LOG`        | `info`                  | Log filter.                                        |

**Set `PUBLIC_BASE_URL` to the URL browsers actually use.** Behind a reverse
proxy at `https://artifacts.example.com`, set it to that — otherwise every
returned link points at localhost. It is applied at response time, so changing
it takes effect immediately for all existing artifacts.

## API

Artifact HTML is sent as the **raw request body** (not multipart, not JSON).
Metadata travels in query parameters.

| Method | Path                  | Purpose                   | Success |
| ------ | --------------------- | ------------------------- | ------- |
| POST   | `/api/artifacts`      | Create                    | 201     |
| PUT    | `/api/artifacts/{id}` | Replace HTML, same URL    | 200     |
| GET    | `/api/artifacts/{id}` | Metadata                  | 200     |
| GET    | `/api/artifacts`      | List, newest first        | 200     |
| DELETE | `/api/artifacts/{id}` | Delete, including history | 204     |
| GET    | `/a/{id}[-{slug}][?version=N]` | Public view               | 200     |
| GET    | `/`                   | Management dashboard      | 200     |
| GET    | `/healthz`            | Health check              | 200     |

Query parameters: `title` and `description` on create/update (on update, only
applied when present — omitting them preserves existing values); `limit`
(default 50, max 200) and `offset` on list; `version` on view.

Artifact JSON:

```json
{
  "id": "3f2a1c6e-8b4d-4a1f-9c2e-7d5b0a9e1f34",
  "title": "Quarterly Metrics",
  "description": "Q3 revenue dashboard",
  "created_at": "2026-08-30T12:04:11.482Z",
  "updated_at": "2026-08-30T12:09:52.117Z",
  "version": 2,
  "size_bytes": 18422,
  "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
  "view_uri": "https://artifacts.example.com/a/3f2a1c6e-8b4d-4a1f-9c2e-7d5b0a9e1f34-quarterly-metrics"
}
```

The optional `{slug}` on the public view route is cosmetic: any text after the
UUID is ignored for routing, so renaming an artifact never breaks an existing
link.

Errors are `{"error": "<code>", "message": "<detail>"}` with status 400
(empty body, malformed UUID), 404 (unknown artifact or version), or 413
(body over `MAX_BODY_BYTES`).

## Dashboard

`GET /` serves a management dashboard: a single self-contained HTML page,
embedded in the binary at compile time, listing every artifact with its title,
description, id, version, size and last update. From it you can open an
artifact, copy its shareable link, expand its version history to reach any
earlier version, and delete it. A search box filters the loaded rows.

It is a client of the same JSON API documented above and adds no endpoints of
its own. Two details worth knowing:

- **Open** uses the relative `/a/{id}`, so it works from whatever host you
  loaded the dashboard from. **Copy link** copies `view_uri`, built from
  `PUBLIC_BASE_URL` — the link to hand to someone else. Behind a proxy these
  differ, which is the point.
- Version history is derived from the current version number; every version
  from 1 to N is addressable, but nothing records when each was published, so
  the list shows version numbers only.

Editing a title or description is deliberately not offered — `PUT` requires the
HTML body and would bump the version, so renaming would need a metadata-only
endpoint that does not exist.

## Storage

One directory per artifact, no database:

```
$DATA_DIR/{uuid}/
  index.html                 # current version
  meta.json                  # metadata
  versions/index.v{N}.html   # every superseded version
```

Writes go through a temp file and `rename`, and new artifacts are assembled in a
`.tmp-{uuid}` directory before being renamed into place, so an interrupted write
never leaves a partial artifact visible. Back up by copying `$DATA_DIR`.

## Security model

The API is **unauthenticated by design** — it is meant to run on a trusted
network or behind a reverse proxy that handles access control. Anyone who can
reach it can create, overwrite, and delete artifacts.

The dashboard at `/` grants no capability the API did not already, but it does
put delete one click away in a browser. If that matters, keep `/` and `/api/*`
behind the proxy's access control and expose only `/a/*`, as below.

Artifacts are served verbatim with no Content-Security-Policy, because they are
intentionally arbitrary HTML with inline scripts. Consequently artifact
JavaScript runs on the same origin as the API and can call it. With no auth this
grants nothing a network peer lacks, but it means **you should only publish HTML
you built or reviewed**.

To expose artifacts publicly while keeping the API private, put a reverse proxy
in front that routes only `/a/*` to the public hostname, and point
`PUBLIC_BASE_URL` at that hostname so links follow.

The view route sets `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, and `Cache-Control: no-cache`. It deliberately
omits `X-Frame-Options` so artifacts stay embeddable. The API allows CORS from
any origin, which grants nothing extra given there is no auth or cookies.

## Skills

`skills/` holds three portable skills for LLM agents, plus a shared visual style
guide:

| Skill                     | Use for                                                        |
| ------------------------- | -------------------------------------------------------------- |
| `simple-artifact-builder` | Single-file HTML artifacts with no build step — the fast path. |
| `web-artifacts-builder`   | React + Tailwind + shadcn/ui artifacts bundled to one file.    |
| `artifact-service`        | Publishing to this service and sharing the link.               |

`skills/STYLE_GUIDE.md` defines the shared Bauhaus-inspired, phone-first
aesthetic both builders follow — paper and ink, bold geometric type, saturated
primary accents — and that the dashboard is built in, so the service looks like
the artifacts it hosts.

`web-artifacts-builder` is adapted from
[anthropics/skills](https://github.com/anthropics/skills) and remains under the
Apache 2.0 license included in its directory.

## Development

```bash
cargo test     # integration tests covering create/update/view/version/delete
cargo clippy --all-targets
```
