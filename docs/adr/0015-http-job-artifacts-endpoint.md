# ADR 0015 — HTTP `GET /jobs/{id}/artifacts`

**Status:** Proposed — not implemented  
**Type:** ADR  
**Date:** 2026-08-03

**Related:**

- [`vd-srv`](../../src/cli/manage/vd-srv/) · HTTP adapter · Artifact Store
- [ADR 0006 — HTTP Transport](0006-http-transport-for-runtime-api.md)
- [ADR 0007 — Runtime API Transports](0007-runtime-api-transports.md)
- [`TRANSPORT.md`](../../src/cli/manage/vd-srv/TRANSPORT.md)
- MCP `list_artifacts` · JSON-RPC `artifact.list`

---

## Motivation

Runtime already exposes job artifact listing on native transports and MCP:

| Surface | Method |
|---------|--------|
| JSON-RPC | `artifact.list` |
| MCP | `list_artifacts` |
| CLI | `vd-srv artifacts <id>` |
| HTTP | **missing** |

HTTP-only clients (local n8n, curl scripts, reverse-proxied tools) can submit and poll Jobs via REST (`POST /planning/*`, `GET /jobs/{id}`), but cannot discover produced artifacts without falling back to shell/JSON-RPC.

HTTP must stay a thin adapter of the Runtime API — this is a **parity gap**, not a new API.

---

## Decision

Add:

```text
GET /jobs/{id}/artifacts
```

on the optional HTTP transport (`vd-srv --http` / `[http]`).

Forward to the existing Runtime method `artifact.list` with `{ "id": "<job id>" }`.

Same semantics and payload shape as JSON-RPC / MCP. No duplicated Artifact Store logic in the HTTP layer.

---

## Contract

### Request

```text
GET /jobs/{id}/artifacts
```

- `id` — Job id (same as `GET /jobs/{id}`).
- No request body.
- Auth: same as other HTTP Runtime routes (out of scope for v1; default bind `127.0.0.1`).

### Response

`200` with the Artifact Store listing for that Job — the same JSON that `artifact.list` returns today (`ArtifactEntry[]`):

```json
[
  {
    "id": "meeting.md",
    "path": "/data/srv/jobs/<job-id>/artifacts/meeting.md",
    "kind": "markdown",
    "producer": "vd-meeting"
  }
]
```

Fields (`id`, `path`, optional `kind` / `producer`) follow the existing store schema; HTTP must not invent a parallel DTO.

### Errors

| Condition | HTTP |
|-----------|------|
| Job not found | `4xx` with `{"error": …}` (same mapping as `GET /jobs/{id}`) |
| Engine / store failure | existing HTTP error mapping for Runtime RPC errors |

### Non-goals

- **No file-byte download** on this route (no `GET /artifacts/{id}/content`, no static file server).
- Local / same-host consumers read `path` from the listing (shared volume or host filesystem).
- Remote byte delivery, signed URLs, and auth are out of scope for this ADR.
- Webhooks, n8n packaging, and enabling HTTP by default are out of scope.

---

## Implementation sketch (when built)

1. Route in `vd-srv` HTTP adapter next to `GET /jobs/{id}`:

   ```text
   GET /jobs/{id}/artifacts  →  artifact.list { id }
   ```

2. OpenAPI: add path under Execution (same document as ADR 0006 / 0007 surface).
3. Docs: `vd-srv` cli.md / TRANSPORT.md route tables.
4. Tests: HTTP integration — empty list, populated `artifacts.json`, unknown job.

No changes to Planner, Executor, or Artifact Store schema.

---

## Consequences

- HTTP clients can finish the Job lifecycle without shell: plan → poll → list artifacts → read paths.
- Closes transport parity for `artifact.list`.
- Keeps download/remote artifact access as a separate future decision.

---

## Success criteria

- `GET /jobs/{id}/artifacts` returns the same payload as JSON-RPC `artifact.list` for the same Job.
- OpenAPI documents the route.
- No new Runtime semantics beyond existing Artifact Store listing.
