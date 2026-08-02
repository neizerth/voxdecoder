# vd-url — CLI surface

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Gates: [RUST.md](RUST.md).

Binary: **`vd-url`**. Capability: **`import-url`** (`use: import-url`).

The CLI and the Runtime capability share the same import library.  
There is exactly one implementation of URL import.

Importer only — URL → `ImportResult` (artifacts). No ASR. No Job planning.

---

## Synopsis

```text
vd-url <COMMAND>

Commands:
  run         Import URL → ImportResult (platform convention; same as import)
  inspect     Metadata only — no audio download (required command)
  validate    Local checks: URL · resolver hint · subtitle policy (no network)
  providers   List resolvers and their capabilities
  doctor      Check external tools (yt-dlp · ffmpeg)
  config      Show effective configuration
  version
  help
```

Global flags follow other process CLIs: `-q` / `--quiet`, `-o json|text`, progress via `vd-progress`.

`run` is kept for consistency with other process CLIs. Semantically it means **import** (not “execute a Job”).

---

## `run`

```bash
vd-url run \
  -i 'https://youtu.be/XXXXXXXXXXX' \
  --output-dir ./out \
  --subtitles prefer
```

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `-i` / `--input` | URL | required | Online media URL (`InputSource.url`) |
| `--output-dir` | path | cwd / work | Directory where artifacts are written |
| `--subtitles` | `ignore` · `prefer` · `require` | `ignore` | Subtitle policy (YouTube) |
| `--provider` | hint | `auto` | Resolver hint (`auto` · `youtube` · `direct` · …) — not a closed CLI enum |
| `--metadata-only` | flag | off | Same as `inspect` (metadata Artifact only) |
| `--overwrite` | flag | off | Replace existing outputs |
| `-q` / `--quiet` | | | Quiet |
| `-o` / `--output` | `text` · `json` | `text` | Report format |

Exit: `0` on success; non-zero if download fails or `--subtitles require` and none available.

### JSON report (filesystem-independent)

JSON reports **artifacts**, not paths:

```json
{
  "ok": true,
  "provider": "youtube",
  "artifacts": [
    { "id": "audio", "kind": "audio" },
    { "id": "metadata", "kind": "metadata" },
    { "id": "subtitle", "kind": "subtitle" }
  ]
}
```

(`audio` omitted when `--metadata-only`.)

Text mode may print human-readable paths under `--output-dir`. Paths are a CLI convenience; the machine contract is artifact ids / kinds.

---

## `inspect`

**Required** command (Desktop and tooling will rely on it).

```bash
vd-url inspect -i 'https://youtu.be/XXXXXXXXXXX'
# equivalent:
vd-url run -i 'https://youtu.be/XXXXXXXXXXX' --metadata-only
```

Resolves provider metadata **without** downloading audio:

- duration
- language (when known)
- subtitle tracks available
- chapters
- title / channel / …

Writes Metadata Artifact (with `import.provider`) and prints a short summary (`-o json` → same artifact-list shape, metadata only).

---

## `validate`

Local / offline checks only — **no network**, no download.

```bash
vd-url validate -i 'https://youtu.be/XXXXXXXXXXX' --subtitles require
```

```text
✓ URL valid
✓ Provider resolved
✓ Subtitles policy supported
```

Fails if the URL is malformed, no resolver matches the hint, or the chosen provider cannot honor the subtitle policy (e.g. `require` on `direct`).

---

## `providers`

Lists resolvers and **capabilities**:

```bash
vd-url providers
```

```text
youtube
  audio
  metadata
  subtitles
  inspect

direct
  audio
  metadata
```

(`inspect` for `direct` is limited — HTTP headers / filename only.)

---

## `doctor`

Checks external tools the import library needs:

```bash
vd-url doctor
```

```text
✓ yt-dlp   /opt/homebrew/bin/yt-dlp  (…version…)
✓ ffmpeg   /opt/homebrew/bin/ffmpeg  (…version…)
```

Respects `VD_YTDLP` / `VD_FFMPEG` when set. Exit non-zero if a required tool is missing.

---

## Artifacts on disk

CLI writes under `--output-dir`:

```text
<output-dir>/
  …artifacts…
```

Layout, filenames, and ids are owned by [`vd-artifact`](../../../crates/vd-artifact/) — not fixed by this CLI.

Text mode may show the resulting paths; JSON does not.

---

## Job step (Executor)

URL is an **InputSource** on `inputs`. Options are import policy only:

```yaml
- use: import-url
  id: imported
  inputs:
    - url: https://youtu.be/XXXXXXXXXXX
  options:
    subtitles: prefer
    # provider: youtube   # optional resolver hint
```

Downstream steps consume `imported` artifacts by id. How Planning inserts this leaf is owned by the Planning API / domain planners — not by this CLI doc.

---

## InputSource (requests)

Prefer shared InputSource — not a separate `source:` type:

```yaml
audio:
  url: https://youtu.be/XXXXXXXXXXX
```

```yaml
inputs:
  - role: room
    url: https://youtu.be/XXXXXXXXXXX
```

`subtitles:` / `provider:` are request or step **options**, not fields inside InputSource.

See [`docs/input-source.md`](../../../../docs/input-source.md).

---

## Related

- Product: [README.md](README.md)
- Layout: [STRUCTURE.md](STRUCTURE.md)
- Gates: [RUST.md](RUST.md)
- Runtime Planning: [`../../manage/vd-srv/`](../../manage/vd-srv/)
