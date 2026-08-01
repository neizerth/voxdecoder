# vd-postprocess CLI

Universal **recipe executor**: named input artifacts + user recipes + execution provider → derived artifacts.  
Standalone CLI **and** `use: postprocess` for the shared Executor.

**Status: implemented.**

Product: [README.md](README.md). Layout: [STRUCTURE.md](STRUCTURE.md). Process: [../README.md](../README.md).

---

## Architecture

```text
CLI flags / Job step (use: postprocess)
              ↓
        vd-postprocess
              ↓
       Derived Artifacts   (registered outputs)
```

Same binary / library for both surfaces. **No recipes → error.**

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-postprocess run` | Apply recipe(s) to named input artifact(s) |
| `vd-postprocess config` | Defaults (provider, progress, …) |
| `vd-postprocess validate` | *(planned)* Check recipe document(s) without invoking provider |

Shorthand (planned): named `--input` / `--recipe` without subcommand inserts `run`.

---

## `run`

```bash
# fails — no recipes
vd-postprocess run --input meeting=meeting.json
# error: no recipes specified

vd-postprocess run \
  --input meeting=meeting.json \
  --input transcript=out.txt \
  --input context=.voxdecoder \
  --recipe ./summary.yaml \
  --recipe ./tasks.yaml \
  --provider openai \
  --model gpt-5 \
  --var audience=Executives \
  --var language=Russian

vd-postprocess run \
  --input meeting=meeting.json \
  --recipe ./my-recipe.yaml \
  --provider process \
  --dry-run --json
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` | — | — | Repeatable `name=path` (or artifact path); **at least one** |
| `-i` | — | — | Sugar: single anonymous / default binding (optional) |
| `--recipe` | `-r` | — | Repeatable recipe path (**required at least one**) |
| `--output-dir` | `-d` | cwd / recipe dirs | Base dir for relative recipe output paths |
| `--provider` | — | config | Execution provider type |
| `--model` | `-m` | — | Model id when provider needs one |
| `--var` | — | — | Repeatable `key=value` → recipe variables |
| `--dry-run` | — | — | Plan only (no provider invoke) |
| `--json` | — | — | With `--dry-run`: plan JSON |
| `--progress` | — | `text` | `text` \| `json` |
| `--quiet` | `-q` | — | No progress |
| `--overwrite` | — | — | Replace existing derived files |

### Job step

```yaml
- use: postprocess
  id: summary
  inputs:
    transcript: transcript
    meeting: meeting
    context: .voxdecoder
    glossary: terms.yml
  options:
    provider:
      type: openai
      model: gpt-5
    recipes:
      - ./summary.yaml
    variables:
      language: Russian
      audience: Executives
      company: VoxDecoder
```

Parallel fan-out (preferred for independent recipes):

```yaml
- use: postprocess
  id: summary
  inputs: { transcript: transcript, meeting: meeting }
  options:
    recipes: [./summary.yaml]
    provider: { type: openai, model: gpt-5 }

- use: postprocess
  id: tasks
  inputs: { transcript: transcript, meeting: meeting }
  options:
    recipes: [./tasks.yaml]
    provider: { type: openai, model: gpt-5 }
```

MCP sends the same Job fragment — never a provider SDK:

```json
{
  "use": "postprocess",
  "inputs": { "meeting": "meeting", "transcript": "transcript" },
  "options": {
    "provider": { "type": "anthropic", "model": "claude-sonnet" },
    "recipes": ["./summary.yaml"],
    "variables": { "audience": "Executives" }
  }
}
```

Note: Job schema today uses `input` / `inputs` as path-or-id lists on steps. Named bindings for postprocess may live under `options.inputs` **or** extend step `inputs` — finalize at implementation; product intent is a **named map**.

---

## Recipe document

User-owned YAML/JSON — a full document, not “just a prompt”. Exact fields harden at implementation; sketch:

```yaml
version: 1
id: summary
name: Meeting summary

inputs:
  transcript:
    required: true
  meeting:
    required: false

variables:
  audience: Engineering

provider:
  temperature: 0.2

prompt: |
  Summarize for {{ audience }}.
  {% if meeting %}Meeting: {{ meeting }}{% endif %}
  Transcript: {{ transcript }}

outputs:
  - id: summary
    path: summary.md
    mime: text/markdown
  - id: decisions
    path: decisions.md
    mime: text/markdown
```

| Block | Role |
|-------|------|
| `id` / `name` | Identity / UX |
| `inputs` | Named artifacts this recipe expects (`ArtifactRef` at runtime) |
| `variables` | Defaults; run-time `--var` / Job `variables` override |
| `provider` | Optional knobs merged into `ExecutionProvider` (temperature, …) |
| `prompt` / body | Provider-specific payload (prompt today; process/HTTP later) |
| `outputs` | **Declared** derived artifacts — one recipe may emit many |

### Output declaration

```yaml
outputs:
  - id: summary
    path: summary.md
    mime: text/markdown
  - id: tasks
    path: tasks.json
    format: json
    schema: ./schemas/task-list.json
```

CLI / Job do not invent paths. Recipe owns outputs; Executor registers them as artifacts.

### Output schema / format

| Field | Role |
|-------|------|
| `format` | Coarse kind (`markdown`, `json`, `csv`, …) |
| `mime` | Optional MIME |
| `schema` | Optional JSON Schema (or similar) for validation |

---

## Behavior

1. Parse flags / Job options → `PostprocessRequest` (named `ArtifactRef` inputs).
2. If `recipes` empty → **exit 2** (`no recipes specified`).
3. Resolve `ExecutionProvider` → load recipes → validate inputs → build **ExecutionPlan**.
4. `--dry-run` → print plan → exit 0 (no provider invoke).
5. Execute plan: each recipe → `RecipeResult { outputs }`; register artifacts.
6. Exit 0 or provider / I/O / validation code.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Provider / I/O / schema validation |
| 2 | No recipes / bad recipe / bad options |
| 3 | Missing required input artifact |

---

## Config

```bash
vd-postprocess config list
vd-postprocess config get provider.type
vd-postprocess config set provider.type ollama
vd-postprocess config path
```

| Key | Default | Description |
|-----|---------|-------------|
| `provider.type` | — | Default execution provider |
| `provider.model` | — | Default model (when applicable) |
| `progress` | `text` | Progress mode |

`$VD_POSTPROCESS_CONFIG` or platform config dir.  
Secrets via environment — not committed recipe files.

Priority: CLI > Job `options` > config > default.

---

## Public contract note

**Recipes are required. ExecutionProvider runs recipes. Outputs are artifacts declared by the recipe (many per recipe OK).**  
`use: postprocess` is a normal DAG capability — backend binding, not “must spawn a binary”.
