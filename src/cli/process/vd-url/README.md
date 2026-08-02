# vd-url — Online Media Import

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Rust gates: [RUST.md](RUST.md).  
Shared crates: [`vd-artifact`](../../../crates/vd-artifact/), [`vd-output`](../../../crates/vd-output/), [`vd-progress`](../../../crates/vd-progress/).  
Runtime integration: [`vd-srv`](../../manage/vd-srv/) (Planning API).  
Input contract: [`docs/input-source.md`](../../../../docs/input-source.md).

**Status: v1 implemented** (CLI + library; UrlResolver shared via [`vd-input`](../../../crates/vd-input/); ADR [0008](../../../../docs/adr/0008-input-resolution-layer.md)). Path: `src/cli/process/vd-url`.

---

## Goal

Import online media into the VoxDecoder platform.

`vd-url` resolves supported URLs into Runtime-ready artifacts.

It never performs transcription, diarization, summarization, or any other media processing.

---

## Core rule

```text
vd-url resolves online media into VoxDecoder artifacts.

It downloads media and metadata.

It optionally downloads subtitles.

It never performs speech recognition
or makes processing decisions.
```

---

## Naming

| Layer | Name |
|-------|------|
| CLI / crate | **`vd-url`** |
| Capability | **`import-url`** (`use: import-url`) |

Same pattern as other process tools: the binary is `vd-url`; the Job DAG never says `use: vd-url`.

The CLI and the Runtime capability share the same import library — exactly one implementation of URL import.

---

## Supported sources (v1)

### YouTube

Full support.

Capabilities:

- audio download
- metadata extraction
- subtitle download (optional)
- metadata-only inspect (no media download)

### Direct media URL

Supports downloading media from a direct URL.

Examples:

```text
https://example.com/audio.mp3
https://example.com/audio.wav
https://example.com/audio.m4a
https://example.com/video.mp4
```

If the source is a video, only the audio stream is extracted.

No metadata discovery beyond what is available from HTTP.

No subtitle support.

---

## Future providers

Planned without changing the CLI surface:

```text
Vimeo
RuTube
VK Video
```

---

## Architecture

```text
InputSource.url
        │
        ▼
      vd-url
   (import-url)
        │
        ▼
    ImportResult
        │
        ├── Audio Artifact
        ├── Metadata Artifact
        └── Subtitle Artifact? (optional)
        │
        ▼
      Artifacts
        │
        ▼
      vd-srv
```

`vd-url` is an **importer library** (plus CLI).

It knows about **Artifacts** and **ImportResult**.

It does **not** know which Planning domain (audio Job, meeting Job, …) will consume them.

```text
URL  →  vd-url  →  Artifacts  →  Runtime Planning API
```

| Surface | Role |
|---------|------|
| **CLI** (`vd-url run` · `inspect` · `doctor`) | Human UX: URL → artifacts (or metadata-only); same library as Runtime |
| **`use: import-url`** | Same library as a capability leaf on the Executor |
| **Planning API** (`vd-srv`) | Resolves `InputSource.url` into an `import-url` step; never owns yt-dlp / HTTP SDKs |

---

## Responsibilities

Owns:

- URL validation
- provider detection
- media discovery
- audio download
- metadata extraction
- subtitle download (where supported)
- artifact generation
- `ImportResult` assembly

Does not own:

- transcription
- subtitle cleanup
- ASR correction
- speaker diarization
- meeting processing
- summarization
- Job execution
- domain planning (audio vs meeting vs …)

---

## Provider abstraction

Every provider implements the same interface.

```text
resolve(request)
  ↓
Audio Artifact
Metadata Artifact
Subtitle Artifact?
  ↓
ImportResult
```

```text
ImportResult
  audio: Artifact      # omitted in metadata-only mode
  metadata: Artifact
  subtitle: Option<Artifact>
  provider: ProviderId
```

The Runtime never needs to know which concrete backend produced the artifacts — only `ImportResult` and artifact kinds.

---

## Artifacts

Full import always produces:

```text
Audio Artifact
Metadata Artifact
```

Optionally:

```text
Subtitle Artifact
```

Metadata-only mode produces:

```text
Metadata Artifact
```

(no audio download).

Whether a Subtitle Artifact replaces transcription is a **Planning API** decision, not an importer decision.

---

## Subtitle policy

Only providers that support subtitles expose this functionality.

**(v1: YouTube)**

Three modes — no more for now:

```text
ignore | prefer | require
```

### Default

```yaml
subtitles: ignore
```

Download audio only.

### Prefer

```yaml
subtitles: prefer
```

```text
subtitles available
  ↓
download subtitles
  ↓
produce Subtitle Artifact
```

If subtitles do not exist: download audio; continue without a Subtitle Artifact.

### Require

```yaml
subtitles: require
```

```text
subtitles unavailable
  ↓
error
```

Useful for automated workflows.

---

## Metadata

Metadata Artifact contains provider-specific information **plus** an import provenance block so later stages know how the Artifact was produced.

### Provenance (always)

```yaml
import:
  provider: youtube      # or direct | …
  # optional: resolver binary / crate version when useful
  # version: "…"
```

### Example (YouTube)

```yaml
import:
  provider: youtube

url: ...
video_id: ...
title: ...
channel: ...
published_at: ...
duration: ...
language: ...
chapters: ...
thumbnail: ...
# subtitle tracks available (inspect / prefer discovery)
subtitles_available: ...
```

### Example (direct URL)

```yaml
import:
  provider: direct

url: ...
filename: ...
mime_type: ...
content_length: ...
```

Metadata is preserved throughout the Runtime pipeline.

---

## InputSource

Online media uses the shared [`InputSource`](../../../../docs/input-source.md) — not a separate `source:` type.

```yaml
audio:
  url: https://youtu.be/...
```

```yaml
# meeting input (domain request)
inputs:
  - role: room
    url: https://youtu.be/...
```

`url` sits beside `path` · `uri` · `artifact` · `blob`. Options such as `subtitles:` live on the Domain Request / Job step options, not inside InputSource itself.

---

## Runtime interaction

The **Planning API** resolves URL-bearing InputSources into an `import-url` capability.

```text
InputSource.url (+ subtitles policy on the request)
  ↓
Planning API
  ↓
use: import-url
  ↓
ImportResult artifacts
  ↓
remainder of the Job DAG
```

Example request fragment:

```yaml
audio:
  url: https://youtu.be/...
# request options
subtitles: prefer
```

Planning behavior (Runtime, **not** `vd-url`):

```text
Subtitle Artifact present
  ↓
may omit transcription

otherwise
  ↓
Audio Artifact → transcription
```

Importers never make execution decisions.

---

## Metadata-only (inspect)

Often you need duration, language, chapters, or subtitle availability **without** downloading audio.

```bash
vd-url inspect 'https://youtu.be/XXXXXXXXXXX'
# or
vd-url run -i 'https://youtu.be/XXXXXXXXXXX' --metadata-only
```

```text
resolve(metadata_only=true)
  ↓
Metadata Artifact
  (no Audio Artifact)
```

Same provider backends; cheaper path for tooling and Planning preflight.

---

## Future: Input Resolvers

`vd-url` is one **Input Resolver** in a wider pattern:

```text
InputSource
  path
  uri
  url
  artifact
  blob
    ↓
  Resolver
    ↓
  Artifact(s)
```

| Input | Resolver (conceptual) |
|-------|------------------------|
| `path` / `file://` | local file (identity / light normalize) |
| `url` | **`vd-url`** / `import-url` |
| `artifact` | Runtime artifact store |
| `blob` | Runtime ingest |

Planning then never special-cases YouTube: it sees `InputSource`, picks a Resolver, receives Artifacts, continues the DAG.

---

## CLI philosophy

`vd-url` is an importer.

Its responsibility ends after producing Runtime artifacts (`ImportResult`).

Everything that follows belongs to the Runtime Planning / Execution APIs.

---

## Success criteria

- Import YouTube media.
- Import direct media URLs.
- Download audio.
- Extract audio from video when necessary.
- Download metadata (including provenance `import.provider`).
- Optionally download subtitles (YouTube): `ignore` · `prefer` · `require`.
- Support metadata-only inspect without audio download.
- Produce VoxDecoder artifacts / `ImportResult` only.
- Never perform transcription.
- Never decide Runtime execution strategy.
- Integrate via `InputSource.url` and capability `import-url` only — no planner-specific coupling.

---

## Related

- InputSource: [`docs/input-source.md`](../../../../docs/input-source.md)
- Runtime: [`docs/runtime.md`](../../../../docs/runtime.md) · [`vd-srv`](../../manage/vd-srv/)
- Process overview: [../README.md](../README.md)
- Media prepare (after import): [../vd-preprocess/](../vd-preprocess/)
