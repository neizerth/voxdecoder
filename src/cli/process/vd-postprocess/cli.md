# vd-postprocess CLI

Universal **recipe-graph executor**: named `ArtifactRef` inputs + user recipes (+ optional runner override) → derived artifacts.  
Standalone CLI **and** `use: postprocess` for the shared Executor.

**Status: implemented (v0)** — flags still accept `--provider` as alias for `--runner` until rename lands.  
**Target:** Recipe owns default Runner + graph; Job only lists recipes and may override. Product: [README.md](README.md).

Layout: [STRUCTURE.md](STRUCTURE.md). Process: [../README.md](../README.md). Platform: [../../../README.md](../../../README.md).

---

## Architecture

```text
CLI flags / Job step (use: postprocess)
              ↓
        vd-postprocess
              ↓
     resolve Runner     CLI > Job > Config > Recipe
              ↓
        build ExecutionPlan
              ↓
     [--dry-run → emit plan → stop]
              ↓
     execute ExecutionPlan  (ExecutionRunner per node)
              ↓
       Derived Artifacts
```

Same binary / library for both surfaces. **No recipes → error.**

### Runner resolution priority

Everywhere the same rule:

```text
CLI
 ↓
Job
 ↓
Config
 ↓
Recipe default
```

Compact form: **`CLI > Job > Config > Recipe`**.

Node-level `runner:` in the graph pins that node (not overridden by Job/CLI house default unless the node omits `runner` and inherits the resolved recipe default).

---

## Overview

| Command | Description |
|---------|-------------|
| `vd-postprocess run` | Apply recipe graph(s) to named `ArtifactRef` input(s) |
| `vd-postprocess config` | Defaults (runner, progress, …) |
| `vd-postprocess validate` | *(planned)* Check recipe document(s) without invoking runner |

Shorthand (planned): named `--input` / `--recipe` without subcommand inserts `run`.

---

## `run`

```bash
# fails — no recipes
vd-postprocess run --input meeting=meeting.json
# error: no recipes specified

# Recipe carries its own runner — Job/CLI need not repeat it
vd-postprocess run \
  --input meeting=meeting.json \
  --input transcript=out.txt \
  --input context=.voxdecoder \
  --recipe ./summary.yaml \
  --recipe ./tasks.yaml \
  --var audience=Executives \
  --var language=Russian

# Optional override: run OpenAI-authored recipe via Ollama
vd-postprocess run \
  --input meeting=meeting.json \
  --recipe ./summary.yaml \
  --runner ollama \
  --model qwen3

vd-postprocess run \
  --input meeting=meeting.json \
  --recipe ./my-recipe.yaml \
  --runner process \
  --dry-run --json
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--input` | — | — | Repeatable `name=path` or `name.artifact=…` → `ArtifactRef`; **at least one** |
| `-i` | — | — | Sugar: single anonymous / default binding (optional) |
| `--recipe` | `-r` | — | Repeatable recipe path (**required at least one**) |
| `--output-dir` | `-d` | cwd / recipe dirs | Base dir for relative recipe output paths |
| `--runner` | — | — | Override recipe default (`CLI > Job > Config > Recipe`; `--provider` = alias in v0) |
| `--model` | `-m` | — | Model id when runner needs one |
| `--var` | — | — | Repeatable `key=value` → recipe **variables** (not secrets) |
| `--dry-run` | — | — | Emit **ExecutionPlan** only (no runner invoke) |
| `--json` | — | — | With `--dry-run`: plan JSON |
| `--progress` | — | `text` | `text` \| `json` |
| `--quiet` | `-q` | — | No progress |
| `--overwrite` | — | — | Replace existing derived files |

### Job step

```yaml
- use: postprocess
  id: summary
  inputs:
    transcript:
      artifact: transcript
    meeting:
      artifact: meeting
    context:
      artifact: .voxdecoder
    glossary:
      artifact: terms.yml
  options:
    recipes:
      - ./summary.yaml
    # runner: optional override (CLI > Job > Config > Recipe)
    # runner:
    #   type: ollama
    #   model: qwen3
    variables:
      language: Russian
      audience: Executives
      company: VoxDecoder
```

Parallel fan-out (preferred for independent recipes):

```yaml
- use: postprocess
  id: summary
  inputs:
    transcript: { artifact: transcript }
    meeting: { artifact: meeting }
  options:
    recipes: [./summary.yaml]

- use: postprocess
  id: tasks
  inputs:
    transcript: { artifact: transcript }
    meeting: { artifact: meeting }
  options:
    recipes: [./tasks.yaml]
    runner: { type: ollama, model: qwen3 }   # one-off override
```

MCP sends the same Job fragment — never a runner SDK:

```json
{
  "use": "postprocess",
  "inputs": {
    "meeting": { "artifact": "meeting" },
    "transcript": { "artifact": "transcript" }
  },
  "options": {
    "recipes": ["./summary.yaml"],
    "variables": { "audience": "Executives" }
  }
}
```

---

## Recipe document

User-owned YAML/JSON — a **portable execution graph**, not “just a prompt”. Target sketch:

```yaml
version: 1
id: summary
name: Meeting summary

# Default runner — travels with the recipe pack
runner:
  type: openai
  model: gpt-5
  temperature: 0.2

inputs:
  transcript:
    required: true
  meeting:
    required: false

variables:
  audience: Engineering
  language: Russian

secrets:
  jira_token: env:JIRA_TOKEN
  # future: vault://… / file:… — never plain literals in packs when avoidable

outputs:
  summary:
    artifact: summary
    type: markdown
  decisions:
    artifact: decisions
    type: markdown

graph:
  - id: summarize
    # runner omitted → inherits Recipe.runner (after CLI > Job > Config > Recipe)
    prompt: |
      Summarize for {{ audience }}.
      {% if meeting %}Meeting: {{ meeting }}{% endif %}
      Transcript: {{ transcript }}
    outputs:
      summary:
        artifact: summary
        type: markdown
      decisions:
        artifact: decisions
        type: markdown
```

| Block | Role |
|-------|------|
| `id` / `name` | Identity / UX |
| `runner` | **Default** `ExecutionRunner` for nodes that omit their own |
| `inputs` | Declared slots (bound at runtime; see unified InputRef) |
| `variables` | Non-secret defaults; `--var` / Job `variables` override |
| `secrets` | Secret refs only (`env:…`, later vault/file) — never mixed into `variables` |
| `outputs` | Recipe-level derived artifacts (named map) |
| `graph` | Execution nodes (always; even a single node) |

### Graph node

Every node has a **Runner** (explicit or inherited):

```yaml
graph:
  - id: summarize
    runner:
      type: openai
      model: gpt-5
    prompt: |
      …

  - id: render
    needs: [summarize]
    runner:
      type: process
      command: render.py
    inputs:
      draft:
        from: summarize.summary
    outputs:
      report:
        artifact: report
        type: markdown
        path: reports/summary.md
```

| Field | Role |
|-------|------|
| `id` | Node id (unique in graph) |
| `runner` | Optional; if absent → resolved Recipe default (`CLI > Job > Config > Recipe`) |
| `needs` | Upstream node ids. **Nodes without `needs` execute in parallel.** |
| `inputs` | Unified InputRef map (artifact **or** `from: node.output`) |
| `outputs` | Node outputs (may feed recipe-level outputs / downstream `from:`) |
| `prompt` / `command` / … | Runner-specific body |
| `foreach` | *(reserved)* Expand one node template into N planned nodes |

### Parallelism

```text
Nodes without needs execute in parallel.
Nodes with needs wait until listed upstream nodes complete.
```

```yaml
graph:
  - id: summarize     # no needs → parallel with tasks / jira
  - id: tasks
  - id: jira
  - id: pack
    needs: [summarize, tasks, jira]
```

### Unified InputRef

One syntax everywhere — Job bindings, recipe decls at runtime, and graph edges:

```yaml
inputs:
  transcript:
    artifact: transcript          # external / Job artifact
  meeting:
    artifact: meeting
    format: markdown              # optional projection
  entities:
    from: extract.entities        # output of another graph node
```

| Form | Meaning |
|------|---------|
| `{ artifact: … }` | Artifact (id or path); optional `format` / `selector` / `segments` |
| `{ from: node.output }` | Output produced by an upstream graph node |

No separate “edge” syntax. Recipe-level `inputs` declare what the pack expects from the Job; node-level `inputs` bind those slots and/or upstream outputs.

### Outputs

Map key = logical name. `artifact` = artifact id registered with the Job. `path` = optional filesystem path.

```yaml
outputs:
  summary:
    artifact: summary
    type: markdown
    path: reports/summary.md    # optional; else derived from artifact id + type
  tasks:
    artifact: tasks
    type: json
    schema: ./schemas/task-list.json
```

| Field | Role |
|-------|------|
| `artifact` | Artifact id (stable handle for downstream steps) |
| `type` | Coarse kind (`markdown`, `json`, `csv`, …) |
| `path` | Optional fixed relative path |
| `mime` | Optional MIME |
| `schema` | Optional JSON Schema (or similar) |

CLI / Job do not invent undeclared outputs. Recipe owns the set; the plan registers them.

### Variables vs secrets

```yaml
variables:
  language: Russian
  audience: Executives

secrets:
  jira_token: env:JIRA_TOKEN
  # future: vault://secret/jira | file:~/.config/… 
```

| Block | Allowed |
|-------|---------|
| `variables` | Plain defaults, audience, language, company — safe to log / show in dry-run |
| `secrets` | Refs only (`env:NAME`, later vault/file) — never echoed in progress / plan by default |

Job may pass `variables`; secret material stays in env / vault, referenced by recipe `secrets`.

### `foreach` (reserved)

Reserve now; implement later. One recipe node expands to N planned nodes (e.g. per participant):

```yaml
graph:
  - id: per_speaker
    foreach:
      artifact: participants/*    # or: participant (binding name)
    runner:
      type: openai
    prompt: |
      Summarize for {{ item.id }}:
      {{ item.transcript }}
    outputs:
      note:
        artifact: "note-{{ item.id }}"
        type: markdown
```

Exact `foreach` shape hardens at implementation; dry-run must show the **expanded** `ExecutionPlan` nodes.

---

## ExecutionPlan

First-class object. Built before any invoke. **`--dry-run` emits it; execute consumes it.**

```rust
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub outputs: Vec<ArtifactOutput>,
}

pub struct ExecutionNode {
    pub id: String,                          // unique in plan (foreach may suffix)
    pub runner: RunnerSpec,                  // fully resolved
    pub needs: Vec<String>,
    pub inputs: BTreeMap<String, InputRef>,  // artifact | from
    pub body: NodeBody,                      // Prompt | Command | Http | Mcp | …
    pub outputs: Vec<ArtifactOutput>,
}

pub struct ArtifactOutput {
    pub artifact: String,                    // artifact id
    pub r#type: String,                      // markdown | json | …
    pub path: PathBuf,
    pub mime: Option<String>,
    pub schema: Option<PathBuf>,
}

pub enum InputRef {
    Artifact { artifact: String, format: Option<String>, /* selector, segments */ },
    From { node: String, output: String },
}
```

Dry-run JSON is this plan (resolved runners, bindings, output paths, parallel groups) — not a vague “would call OpenAI”.

---

## ExecutionRunner

Product name **Runner**; Rust trait **`ExecutionRunner`**:

```rust
pub trait ExecutionRunner {
    fn execute(&self, node: &ExecutionNode, ctx: &RunContext) -> Result<NodeResult, PostprocessError>;
}

// implementations (examples)
struct OpenAIRunner;
struct AnthropicRunner;
struct OllamaRunner;
struct ProcessRunner;
struct HttpRunner;
struct McpRunner;
struct StubRunner;   // CI
```

| Family | `runner.type` → impl |
|--------|----------------------|
| **LLM** | `openai` → `OpenAIRunner`, `anthropic`, `gemini`, `ollama`, `qwen`, `gigachat`, … |
| **Process** | `process` / `python` / `bash` → `ProcessRunner` |
| **Service** | `http` → `HttpRunner`, `grpc`, `mcp` → `McpRunner` |
| **Future** | `wasm`, `plugin`, … |
| **CI** | `stub` → `StubRunner` |

v0 module may still be named `provider.rs`; target rename: `runner.rs` + `ExecutionRunner`.

---

## Behavior

1. Parse flags / Job options → `PostprocessRequest` (named `ArtifactRef` / InputRef inputs).
2. If `recipes` empty → **exit 2** (`no recipes specified`).
3. Load recipes → resolve runners (**`CLI > Job > Config > Recipe`**) → expand `foreach` (when implemented) → validate inputs → build **`ExecutionPlan`**.
4. `--dry-run` → print plan → exit 0 (no `ExecutionRunner` invoke).
5. Execute plan: schedule nodes (**no `needs` → parallel**); each node → `ExecutionRunner::execute`; register `ArtifactOutput`s.
6. Exit 0 or runner / I/O / validation code.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or dry-run |
| 1 | Runner / I/O / schema validation |
| 2 | No recipes / bad recipe / bad options |
| 3 | Missing required input artifact |

---

## Config

```bash
vd-postprocess config list
vd-postprocess config get runner.type
vd-postprocess config set runner.type ollama
vd-postprocess config path
```

| Key | Default | Description |
|-----|---------|-------------|
| `runner.type` | — | Config layer in `CLI > Job > Config > Recipe` (`provider.type` alias in v0) |
| `runner.model` | — | Default model (when applicable) |
| `progress` | `text` | Progress mode |

`$VD_POSTPROCESS_CONFIG` or platform config dir.  
Secrets via `secrets:` + environment / vault — not committed as plain values in recipe packs.

**Runner resolution priority: `CLI > Job > Config > Recipe`.**

---

## Public contract note

**Recipes are required and portable (default Runner + graph + outputs + secrets refs). Job selects recipes and may override Runner. Inputs are unified InputRef. Outputs declare `artifact` + `type`. Dry-run emits ExecutionPlan. Nodes without `needs` run in parallel.**  
`use: postprocess` is a normal DAG capability — backend binding, not “must spawn a binary”.
