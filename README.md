# VoxDecoder

License: [MIT](LICENSE)

## CLI apps

### Transcribation

See [cli/transcribe/](cli/transcribe/) — separate CLIs per model (not a universal adapter).

- [vd-giga](cli/transcribe/vd-giga/) — GigaAM ([structure](cli/transcribe/vd-giga/STRUCTURE.md), [cli](cli/transcribe/vd-giga/cli.md))
- vd-whisper — Whisper (TBD)

### Diarization

- vd-dia-hf. hugging face
- vd-dia-pn. pyanote

### vd-srv: background process

Background queue manager — [cli/vd-srv/](cli/vd-srv/)
