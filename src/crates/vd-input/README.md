# vd-input

Input source abstraction — unified handling of media files, URLs, and metadata.

Provides `InputSource` struct used by all VoxDecoder CLIs to represent a single input:
- **media path** (audio/video file) or **URL** (remote stream)
- **role** (audio, participant, context, room, etc.)
- **metadata** (participant label, gender, purposes, subtitles URI)

## Public API

```rust
pub struct InputSource {
    pub role: InputRole,
    pub path: PathBuf,           // file path (empty if url set)
    pub url: Option<String>,      // remote URI (mutually exclusive with path)
    pub participant: Option<String>,
    pub purposes: Vec<InputPurpose>,  // transcript | timeline
    pub subtitles: Option<String>,
    pub provider: Option<String>,     // "youtube" | "zoom" | ...
}

pub enum InputRole { Audio, Participant, Room, Context }
pub enum InputPurpose { Transcript, Timeline }
```

## Design notes

- **Mutually exclusive**: either `path` OR `url`, never both.
- **Cyrillic preservation**: `participant` labels keep original script (never transliterated).
- **No inline content**: URL inputs cannot carry binary data. Context documents must be files.
- **Flexible metadata**: `purposes` vector supports mixed transcription + timeline needs; empty = defaults apply.
