# vd-url — project layout

Rust crate: **online media importer** — domain library **and** CLI surface for resolving URLs into Runtime artifacts. Planned capability: `use: import-url` on the shared Executor.

**Status: v1 implemented.** Path: `src/cli/process/vd-url`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [`docs/input-source.md`](../../../../docs/input-source.md) · [`vd-srv`](../../manage/vd-srv/) · [../vd-preprocess/](../vd-preprocess/)

---

## Philosophy

```text
InputSource.url
  +
Provider
  +
Subtitle policy
  ↓
ImportResult
  (audio? · metadata · subtitle?)
```

Not a downloader toy and not an ASR frontend.

- **Import only** — no transcription, diarization, fix-*, merge, or Job scheduling.
- **Provider plug-ins** — YouTube and direct URL in v1; Vimeo / RuTube / VK later without CLI churn.
- **`ImportResult` is the product** — Planning consumes that structure / artifact ids, not raw yt-dlp JSON.
- **CLI ≠ capability name** — binary `vd-url`; Job leaf `use: import-url`; same library.
- **No planner coupling** — Planning API resolves `url` → `import-url`; domain Jobs live in their own docs.

Product: [README.md](README.md).

---

## Unified capability contract

Every capability in VoxDecoder shares one shape:

```text
Inputs + Options  →  Capability  →  Artifacts
```

| Capability | In | Out |
|------------|----|-----|
| **`import-url`** *(planned)* | `InputSource.url` + subtitle policy + provider | `ImportResult` |
| `preprocess` | media + filters | prepared media |
| `transcribe` | audio | transcript |
| `fix-*` | transcript | transcript |
| `diarize` | audio | timeline |
| `meeting-merge` | tracks + timeline + model | meeting |

Planning / MCP stay universal: they never learn “YouTube” vs “direct HTTP” — only InputSource, `ImportResult`, and options.

---

## Non-goals

- Choosing whether Runtime skips ASR (Planning API reads Subtitle Artifact presence)
- Cleaning or aligning downloaded subtitles (`fix-*` / future subtitle capability)
- Hosting long-running downloads as a second Runtime
- Flat flag soup without a provider model

---

## Planned tree

```text
vd-url/
├── README.md
├── STRUCTURE.md          ← this file
├── cli.md
├── RUST.md
├── Cargo.toml            # workspace member (when implemented)
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli/              # clap: run · inspect · validate · providers · doctor · config
│   ├── import/           # resolve request → ImportResult
│   │   ├── mod.rs
│   │   ├── request.rs    # UrlImportRequest · SubtitlePolicy
│   │   ├── result.rs     # ImportResult { audio?, metadata, subtitle? }
│   │   └── detect.rs     # provider from URL
│   ├── provider/         # Provider trait + youtube · direct · stub
│   │   ├── mod.rs
│   │   ├── youtube.rs
│   │   ├── direct.rs
│   │   ├── stub.rs
│   │   └── tools.rs      # VD_YTDLP · VD_FFMPEG · doctor
│   ├── artifact/         # write Audio / Metadata / Subtitle sidecars
│   └── config/
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
└── proto/                # none (CLI / Job options only)
```

---

## Domain types (planned)

```text
UrlImportRequest
  url: String
  provider: Option<ProviderId>     # auto | youtube | direct | …
  subtitles: SubtitlePolicy        # ignore | prefer | require
  metadata_only: bool             # inspect: skip audio download
  output_dir: Option<PathBuf>
  overwrite: bool

SubtitlePolicy
  Ignore | Prefer | Require

ImportResult
  audio: Option<Artifact>         # None when metadata_only
  metadata: Artifact              # always; includes import.provider
  subtitle: Option<Artifact>
  provider: ProviderId
```

Capability leaf (`use: import-url`) — URL on `inputs`, policy on `options`:

```yaml
use: import-url
id: imported
inputs:
  - url: https://youtu.be/...
options:
  subtitles: prefer          # ignore | prefer | require
  # provider: youtube        # optional resolver hint
  # metadata_only: false
```

Outputs: `ImportResult` registered as artifacts (`audio` when present, `metadata`, `subtitle` when present).

---

## Provider interface

```text
trait MediaProvider {
  id() -> ProviderId
  supports(url) -> bool
  resolve(request) -> ImportResult   # single return; not loose artifacts
}
```

| Provider (v1) | Audio | Metadata | Subtitles | Inspect |
|---------------|-------|----------|-----------|---------|
| `youtube` | yes | rich | optional | yes |
| `direct` | yes (extract audio from video) | HTTP headers / filename | no | limited |

Detection order: explicit `--provider` → scheme/host heuristics → `direct` fallback for bare media URLs.

---

## Planning API (not this crate)

`vd-url` does **not** document default audio Jobs or meeting graphs.

[`vd-srv`](../../manage/vd-srv/) Planning API resolves `InputSource.url` → `use: import-url` → consumes `ImportResult`. How that step is inserted into a particular domain Job is owned by that planner’s docs.

---

## Dependencies (planned)

| Tool / crate | Role |
|--------------|------|
| yt-dlp (or equiv.) | YouTube audio / metadata / subtitles |
| ffmpeg | extract audio from direct video URLs |
| `vd-artifact` | artifact kinds / paths |
| `vd-progress` | download progress |
| `vd-output` | quiet / JSON plan |

External binaries: document `VD_YTDLP` / `VD_FFMPEG` (same spirit as preprocess).

---

## Tests

| Layer | Focus |
|-------|-------|
| `tests/unit/` | URL detect · subtitle policy · metadata shape |
| `tests/integration/` | provider stubs → artifact files on disk |
| `tests/e2e/` | real yt-dlp / HTTP when `VD_URL_E2E=1` (ignored by default) |

---

## Status checklist

- [x] Crate in workspace
- [x] YouTube provider
- [x] Direct URL provider
- [x] CLI `run` · `inspect` · `validate` · `providers` · `doctor` · `config`
- [x] `ImportResult` + metadata provenance (`import.provider`)
- [x] `Capability::ImportUrl` + Executor binder
- [x] Planning API: `InputSource.url` → resolve via `vd-input` → static Job (ADR 0008)
