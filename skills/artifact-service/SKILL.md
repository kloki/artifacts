---
name: artifact-service
description: Publish, update, list, and delete self-contained HTML artifacts on a self-hosted artifact service via its REST API, and give the human a shareable view link. Use after building an HTML artifact, or when asked to share, host, republish, or manage a hosted artifact.
---

# Using the Artifact Service

The artifact service hosts self-contained HTML files and serves each one at a
stable public URL. Publishing turns a local file the human cannot open into a
link they can.

**The link is the deliverable.** A local path like `./bundle.html` is not — it
lives in your working directory, not their browser.

## Configuration

The service base URL comes from the `ARTIFACTS_URL` environment variable:

```bash
echo "${ARTIFACTS_URL:?set ARTIFACTS_URL to the artifact service base URL}"
```

If it is unset, ask the human for the base URL rather than guessing. All
examples below assume it is set (e.g. `https://artifacts.example.com`).

There is no authentication — the service is expected to run on a trusted network
or behind a reverse proxy.

## Publish a new artifact

`POST /api/artifacts` with the **raw HTML as the request body**. This is not a
multipart or JSON upload — send the file bytes directly. Metadata is optional
and goes in query parameters (URL-encode the values).

```bash
curl -sS -X POST \
  -H 'Content-Type: text/html' \
  --data-binary @index.html \
  "$ARTIFACTS_URL/api/artifacts?title=Quarterly%20Metrics&description=Q3%20revenue%20dashboard"
```

Response `201`:

```json
{
  "id": "3f2a1c6e-8b4d-4a1f-9c2e-7d5b0a9e1f34",
  "title": "Quarterly Metrics",
  "description": "Q3 revenue dashboard",
  "created_at": "2026-08-30T12:04:11.482Z",
  "updated_at": "2026-08-30T12:04:11.482Z",
  "version": 1,
  "size_bytes": 18422,
  "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
  "view_uri": "https://artifacts.example.com/a/3f2a1c6e-8b4d-4a1f-9c2e-7d5b0a9e1f34"
}
```

Capture both fields you will need again:

```bash
RESPONSE=$(curl -sS -X POST -H 'Content-Type: text/html' \
  --data-binary @index.html "$ARTIFACTS_URL/api/artifacts?title=My%20Page")
ARTIFACT_ID=$(printf '%s' "$RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
VIEW_URI=$(printf '%s' "$RESPONSE" | grep -o '"view_uri":"[^"]*"' | cut -d'"' -f4)
```

(Use `jq -r .view_uri` instead if `jq` is available.)

## Update an existing artifact

`PUT /api/artifacts/{id}` replaces the HTML. **The `view_uri` never changes** —
any link already shared keeps working and now shows the new content. The version
counter increments and the previous version stays retrievable.

```bash
curl -sS -X PUT \
  -H 'Content-Type: text/html' \
  --data-binary @index.html \
  "$ARTIFACTS_URL/api/artifacts/$ARTIFACT_ID"
```

Title and description are only changed when you pass them; omitting them
preserves the existing values.

Always prefer updating over re-publishing when iterating on the same artifact —
re-publishing creates a second artifact and leaves the human holding a stale
link.

## View and version history

The human opens `view_uri` directly in a browser. Older versions remain
addressable:

```
https://artifacts.example.com/a/{id}             # current version
https://artifacts.example.com/a/{id}?version=1   # first version
```

Requesting a version that never existed returns `404`.

## Inspect, list, and delete

```bash
# Metadata for one artifact
curl -sS "$ARTIFACTS_URL/api/artifacts/$ARTIFACT_ID"

# Newest first; limit defaults to 50, max 200
curl -sS "$ARTIFACTS_URL/api/artifacts?limit=20&offset=0"

# Permanent, including all version history
curl -sS -X DELETE "$ARTIFACTS_URL/api/artifacts/$ARTIFACT_ID"   # 204, empty body
```

Deletion cannot be undone — confirm with the human before deleting anything you
did not just create.

## Telling the human where to look

After publishing or updating, state the URL plainly on its own line as the last
thing you say:

> Published your dashboard: https://artifacts.example.com/a/3f2a1c6e-8b4d-4a1f-9c2e-7d5b0a9e1f34
>
> Open it in a browser to view. Send me changes and I'll update it at the same link.

After an update, say the link is unchanged and now serves the new version — that
is exactly the information the human needs to decide whether to re-share it.
Never present a filesystem path as the way to view an artifact.

## Endpoint summary

| Method | Path                  | Purpose                | Success |
| ------ | --------------------- | ---------------------- | ------- |
| POST   | `/api/artifacts`      | Publish new            | 201     |
| PUT    | `/api/artifacts/{id}` | Replace HTML, keep URL | 200     |
| GET    | `/api/artifacts/{id}` | Metadata               | 200     |
| GET    | `/api/artifacts`      | List (paginated)       | 200     |
| DELETE | `/api/artifacts/{id}` | Delete permanently     | 204     |
| GET    | `/a/{id}[?version=N]` | Human-facing view      | 200     |
| GET    | `/healthz`            | Service health         | 200     |

Query parameters: `title`, `description` on POST/PUT; `limit`, `offset` on list;
`version` on view.

## Errors

Errors return `{"error": "<code>", "message": "<detail>"}`.

| Status                   | Meaning                      | Fix                                                       |
| ------------------------ | ---------------------------- | --------------------------------------------------------- |
| 400                      | Empty body or malformed UUID | Send the HTML as the raw body; check the ID               |
| 404                      | No such artifact or version  | Verify the ID; list artifacts to find it                  |
| 413                      | HTML exceeds the size limit  | Inline fewer/smaller assets; compress embedded images     |
| 000 / connection refused | Service unreachable          | Check `ARTIFACTS_URL` and `curl "$ARTIFACTS_URL/healthz"` |

## Notes

- Artifacts must be **fully self-contained** — the service stores exactly one
  HTML file per artifact and serves no sibling assets. Inline all CSS, JS, and
  images. Build with the `simple-artifact-builder` or `web-artifacts-builder`
  skill.
- Artifact JS runs on the same origin as this unauthenticated API. Only publish
  HTML you built or reviewed.
