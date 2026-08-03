# vd-text-py — Natasha/razdel Sidecar for VoxDecoder

Linguistic infrastructure sidecar providing tokenization, sentence segmentation, and morphological analysis via [Natasha](https://natasha.ai/) and [razdel](https://github.com/natasha/razdel).

## Overview

`vd-text-py` is a long-lived subprocess spawned by `vd-text` (Rust). It communicates via **file-based IPC** (input/output files, arguments) — matching [vd-pipeline's subprocess model](../../src/cli/process/vd-pipeline/src/exec/subprocess.rs).

**Not a hard dependency.** Features gracefully degrade if the sidecar is unavailable (missing Python, missing packages, spawn failure). Preserved for vd-fix-asr's ADR 0010 guarantee of fully-local operation without Python.

## Installation

```bash
cd python/vd-text-py
pip install -e .
```

Or with dev tools:
```bash
pip install -e ".[dev]"
```

## Usage

### Command-line

```bash
vd-text-py -i input.txt -o output.json -op tokenize
vd-text-py -i input.txt -o output.json -op sentence_split
vd-text-py -i input.txt -o output.json -op morph
```

**Arguments:**
- `-i, --input` : Path to input text file (UTF-8)
- `-o, --output` : Path to output JSON file
- `-op, --operation` : Operation type: `tokenize` (default) | `sentence_split` | `morph`

### Output Format

All operations produce JSON-formatted results.

#### tokenize
```json
{
  "operation": "tokenize",
  "tokens": [
    {"text": "Hello", "start": 0, "end": 5},
    {"text": "world", "start": 6, "end": 11}
  ]
}
```

#### sentence_split
```json
{
  "operation": "sentence_split",
  "sentences": [
    {"text": "First sentence.", "start": 0, "end": 15},
    {"text": "Second sentence.", "start": 16, "end": 32}
  ]
}
```

#### morph
```json
{
  "operation": "morph",
  "analyses": [
    {
      "text": "привет",
      "grammemes": ["NOUN", "inan", "masc"],
      "normalized": "привет",
      "pos": "NOUN"
    }
  ]
}
```

### Error Handling

On error, output contains:
```json
{
  "operation": "tokenize",
  "error": "Error message here"
}
```

Exit code: 1 on error, 0 on success.

## Architecture

**Single long-lived process** (spawned once, reused across calls) — model-load cost amortized.

**Modules:**
- `tokenizer.py` — Natasha-based tokenization
- `sentence_splitter.py` — razdel sentence boundaries
- `morphology.py` — Natasha MorphVocab analysis
- `main.py` — File-based IPC protocol, CLI dispatch

## Integration with vd-text (Rust)

vd-text provides a Rust wrapper (`linguistics` module) that:
1. Spawns vd-text-py once (lazy init)
2. Writes input file
3. Invokes vd-text-py with `-i` / `-o` / `-op`
4. Reads JSON output, deserializes to Rust types
5. Reuses process for subsequent operations

Fallback: If spawn fails or output is malformed, features skip gracefully without crashing.

## Testing

```bash
pytest
pytest --cov
```

## Dependencies

- `natasha>=1.0.0` — Tokenization, morphology
- `razdel>=0.5.0` — Sentence segmentation
- `pydantic>=2.0.0` — JSON serialization (models)

Development:
- `pytest>=7.0.0`
- `pytest-cov>=4.0.0`

## Performance

First call (model load): ~500-1000ms
Subsequent calls: ~1-10ms (I/O dominated)

For high-volume use, keep the process long-lived. Single-spawn-per-operation would be prohibitively slow.

## Related

- [ADR 0013 — Local Linguistic Infrastructure](../../docs/adr/0013-local-linguistic-infrastructure.md)
- [vd-text crate](../../src/crates/vd-text/README.md)
