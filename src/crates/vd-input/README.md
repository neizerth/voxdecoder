# vd-input

Input resolution layer (ADR 0008): `InputSource` → `ResolvedInput`.

Planners consume [`ResolvedInput`] artifacts. They do not consume user sources directly — `resolve()` is the only path from a raw user-supplied source to a Runtime-host path.

## Public API

```rust
// What the user/caller supplies — exactly one of the five fields (XOR).
pub struct InputSource {
    pub path: Option<PathBuf>,
    pub uri: Option<String>,      // e.g. "file://..."
    pub url: Option<String>,      // http(s) only
    pub artifact: Option<String>, // Runtime Job Store artifact id
    pub blob: Option<String>,     // inline content, materialized to a temp file
}

// What planners consume — always a real path on the Runtime host.
pub struct ResolvedInput {
    pub kind: SourceKind,             // File | Url | Artifact | Blob
    pub audio: Option<PathBuf>,
    pub metadata: Option<PathBuf>,
    pub subtitle: Option<PathBuf>,
    pub provider: Option<String>,     // set for Url kind, e.g. "youtube"
}

pub fn resolve(
    source: &InputSource,
    ctx: &ResolveContext<'_>,
    artifact_lookup: Option<&dyn Fn(&str) -> Result<PathBuf, String>>,
) -> Result<ResolvedInput, InputError>
```

## Design notes

- **XOR, not role-based**: `InputSource` has no `role` field. Role (audio/participant/context/room) is a higher-level concept owned by `vd-meeting::model::InputSource` (a distinct, unrelated type) — do not confuse the two.
- **`validate_xor()`**: exactly one of `path`/`uri`/`url`/`artifact`/`blob` must be set; anything else is `InputError::Invalid`.
- **`url`** must be `http://` or `https://`; other schemes go through `uri` (currently only `file://` is supported there).
- **`artifact`** requires an `artifact_lookup` callback (Runtime Job Store) — resolving an artifact input without one is an error.
- **`blob`** is written to `data_dir/inputs/blob-{nonce}.bin` before resolution — inline content never reaches a Job as a literal string.
