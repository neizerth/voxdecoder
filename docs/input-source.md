# InputSource (shared)

**Status:** living note (ADR [0008](adr/0008-input-resolution-layer.md))  
**Crate:** [`vd-input`](../src/crates/vd-input/)  
**Used by:** Runtime Planning API — MCP, Desktop, CLI, REST, Web — and Domain Requests.

An **InputSource** is how a client points the Runtime at media or prior artifacts.  
Same shape everywhere; the gateway does not invent a private upload protocol.

```text
InputSource

  path       filesystem path visible to the Runtime / host
  uri        file:// · other allowed schemes (Runtime policy)
  url        http(s) online media
  artifact   ArtifactId already known to the Runtime
  blob       small inline payload (e.g. MCP binary / base64) — Runtime stores it
```

Exactly one of these fields is set per InputSource (XOR).

## Resolution

```text
InputSource
        │
        ▼
   vd-input (Resolver)
        │
        ▼
   ResolvedInput
     audio · metadata · subtitle?
        │
        ▼
   Planner  →  Job  →  Executor
```

| Field | Resolver |
|-------|----------|
| `path` / `file://` | File |
| `url` | URL ([`vd-url`](../src/cli/process/vd-url/) / UrlResolver) |
| `artifact` | Artifact store |
| `blob` | Blob ingest |

**Planners never see `InputSource.url`.** They receive artifact paths from `ResolvedInput`.

Import options (`subtitles: ignore|prefer|require`, resolver hint, …) live on the Domain Request / `ResolveContext` — **not** inside InputSource.

## Examples

```yaml
audio:
  path: /work/meeting.wav

audio:
  url: https://youtu.be/...
# request options:
# subtitles: prefer
```

```yaml
inputs:
  - role: room
    url: https://youtu.be/...
  - role: context
    path: ./docs
```

## Rules

* Resolution belongs to the **Runtime** (`vd-input` via Planning API), not to frontends or domain planners.
* `blob` is for small payloads; do not treat MCP as a bulk file store.
* Online import provenance (`import.provider`, …) lives on the Metadata Artifact.
* `uri` vs `url`: prefer **`url`** for online media; keep **`uri`** for generic schemes (especially `file://`).

Related: [ADR 0008](adr/0008-input-resolution-layer.md) · [`docs/runtime.md`](runtime.md) · [`vd-input`](../src/crates/vd-input/) · [`vd-srv`](../src/cli/manage/vd-srv/).
