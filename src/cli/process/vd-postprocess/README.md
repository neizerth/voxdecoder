# vd-postprocess — recipe-graph executor for derived artifacts

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-pipeline](../vd-pipeline/) · [vd-meeting](../vd-meeting/) · [vd-preprocess](../vd-preprocess/).  
Shared crates (planned): [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented (v0).** Workspace member: `src/cli/process/vd-postprocess`. Default runner: `stub` (CI / dry pipelines). OpenAI / Ollama / process / HTTP / MCP: typed but not wired yet.  
**Target contract below** (Recipe = portable graph + Runner; Job only selects recipes and may override). Current code still uses the older `provider` field name — migrate to `runner` without changing the Job/recipe *idea*.

## Core rule

```text
vd-postprocess is a universal recipe-graph executor.

ArtifactRef(s) + Recipe (+ optional Runner override) → Derived Artifacts.

It has no built-in recipes.
Without recipes, it does nothing (and errors).
```

> **vd-postprocess produces new artifacts from existing ones by user recipes.**  
> The CLI does not know about “summary”, “tasks”, “jira”, or “decisions”. Those are recipe files the user (or company) owns. One binary; many corporate packs.  
> **Runner** means *who executes a graph node* — LLM, local process, HTTP, MCP tool — not “chat API only”.

## Contract

```text
ArtifactRef(s)
      +
Recipe  (portable: owns default Runner + execution graph + outputs)
      +
optional Runner override  (Job / CLI / config)
      ↓
Derived Artifacts
```

| Surface | Role |
|---------|------|
| **CLI** (`vd-postprocess run`) | Human UX: named `ArtifactRef` inputs + recipes (+ optional runner override) |
| **`use: postprocess`** | Same implementation, scheduled by [`vd-pipeline`](../vd-pipeline/) Executor |
| **MCP / `vd-srv`** | Submit a Job step; never own runner SDKs |

`vd-postprocess` knows nothing about meetings, ASR engines, or speaker identity. Planners only add Job step(s) with `options.recipes` (+ optional `runner` override and `variables`).

---

## Recipe is the only portable unit

Job does **not** own the runner. Recipe does.

```yaml
# Job step — “run these recipes”
- use: postprocess
  id: summary
  inputs:
    meeting:
      artifact: meeting
    transcript:
      artifact: transcript
  options:
    recipes:
      - ./summary.yaml
```

Inside the recipe (travels with the pack):

```yaml
# summary.yaml — fully portable
version: 1
id: summary

runner:
  type: openai
  model: gpt-5

inputs:
  meeting:
    required: true
  transcript:
    required: true

outputs:
  summary:
    type: markdown
  decisions:
    type: markdown

graph:
  - id: main
    prompt: |
      Summarize for {{ audience }}.
      Meeting: {{ meeting }}
      Transcript: {{ transcript }}
```

Same recipe elsewhere with a different backend:

```yaml
runner:
  type: process
  command: python render.py
```

```yaml
runner:
  type: mcp
  tool: jira.create
```

**Moving a recipe between projects does not require editing the Job.** Job only says *which* recipes to run.

---

## Runner override (Job / CLI / config)

Recipe carries the **default** runner. Callers may override. **One rule everywhere:**

```text
Runner resolution priority

CLI
 ↓
Job
 ↓
Config
 ↓
Recipe default
```

Compact: **`CLI > Job > Config > Recipe`**.

Example: run `summary.yaml` (written for OpenAI) via Ollama once:

```yaml
options:
  recipes:
    - ./summary.yaml
  runner:
    type: ollama
    model: qwen3
```

```bash
vd-postprocess run \
  --input meeting.artifact=meeting.json \
  --recipe ./summary.yaml \
  --runner ollama \
  --model qwen3
```

Override replaces the default runner for that invoke; recipe body / graph stay the same. Node-level `runner:` pins that node only.

---

## Inputs are ArtifactRef

Not a bare id string:

```yaml
# ❌ too thin — no room to grow
inputs:
  meeting: meeting
```

```yaml
# ✅ ArtifactRef — extensible
inputs:
  meeting:
    artifact: meeting
  transcript:
    artifact: transcript
    format: markdown
  # future:
  # transcript:
  #   artifact: transcript
  #   selector:
  #     participant: alice
  #   segments:
  #     from: 10m
  #     to: 15m
```

CLI sugar stays ergonomic:

```bash
--input meeting=meeting.json          # → { artifact: <path-or-id> }
--input transcript.artifact=out.txt   # explicit
```

Primary path sugar (`-i FILE`) may map to a default binding when only one input is needed — convenience, not the model.

---

## Recipe = execution graph

A recipe is not “one prompt → one file”. It is an **execution graph**. Even when the graph has a single node, the model is already graph-shaped — so multi-step recipes do not need a second schema later.

```text
Recipe
  └── graph
        ├── node A  (LLM extract → JSON)
        ├── node B  (HTTP / MCP call)
        └── node C  (process → Markdown)
```

Every node has a **Runner** (explicit, or inherited from the resolved Recipe default). Nodes are not “prompt-only”:

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
```

If `runner` is omitted on a node → inherit Recipe default after **`CLI > Job > Config > Recipe`**.

### Parallelism

**Nodes without `needs` execute in parallel.** Nodes with `needs` wait for listed upstream ids.

```yaml
graph:
  - id: summarize
  - id: tasks
  - id: jira
  - id: pack
    needs: [summarize, tasks, jira]
```

### Unified InputRef

Same shape for Job bindings and graph edges — artifact **or** upstream output:

```yaml
inputs:
  transcript:
    artifact: transcript
  meeting:
    artifact: meeting
  entities:
    from: extract.entities
```

### `foreach` (reserved)

Expand one node template into N planned nodes (e.g. per participant). Dry-run must show the expanded **ExecutionPlan**. Exact shape hardens at implementation — see [cli.md](cli.md#foreach-reserved).

Terminology: **Recipe** is the product name. Avoid “Workflow” here — that word belongs to Job DAGs / planners. Inside a recipe we say **graph** / **nodes**.

---

## ExecutionPlan

First-class. Built before invoke; **`--dry-run` emits it; execute consumes it.**

```rust
ExecutionPlan {
    nodes: Vec<ExecutionNode>,    // resolved runners, needs, InputRefs, bodies
    outputs: Vec<ArtifactOutput>, // artifact id + type + path
}
```

Not a side log — the plan *is* what runs. Details: [cli.md](cli.md#executionplan).

---

## Outputs are declared explicitly

Map key = logical name. `artifact` = id. `path` = optional filesystem location.

```yaml
outputs:
  summary:
    artifact: summary
    type: markdown
    path: reports/summary.md
  tasks:
    artifact: tasks
    type: json
    schema: ./schemas/task-list.json
```

One recipe → many registered artifacts. The Executor treats them like any other step outputs.

---

## Variables vs secrets

```yaml
variables:
  language: Russian
  audience: Executives

secrets:
  jira_token: env:JIRA_TOKEN
  # future: vault://… 
```

`variables` are safe defaults (may appear in dry-run). `secrets` are refs only — never plain API keys in packs, never mixed into `variables`.

---

## Runner catalog (`ExecutionRunner`)

Product name **Runner**; Rust trait **`ExecutionRunner`** with typed impls (`OpenAIRunner`, `ProcessRunner`, `HttpRunner`, `McpRunner`, …).

Anything that can execute a graph node:

| Family | `runner.type` (examples) |
|--------|--------------------------|
| **LLM** | `openai`, `anthropic`, `gemini`, `ollama`, `qwen`, `gigachat`, … |
| **Process** | `process`, `python`, `bash` |
| **Service** | `http`, `grpc`, `mcp` |
| **Future** | `wasm`, `plugin`, … |
| **CI** | `stub` |

```yaml
runner:
  type: ollama
  model: qwen3
```

```yaml
runner:
  type: process
  command: ./bin/my-renderer
```

```yaml
runner:
  type: mcp
  tool: jira.create
```

Auth / base URL / API keys via `secrets` + env + config — never baked into Meeting Model. MCP picks recipes (+ optional runner override) in Job JSON; it does not implement clients.

**Naming:** prefer **Runner** / `ExecutionRunner` in product docs and new APIs. Older code / flags may still say `provider` / `ExecutionProvider` until renamed — same concept.

---

## Not the last step — a normal capability

`postprocess` is **not** glued to the end of a pipeline. It is a regular DAG node. One transcript (or meeting) can fan out to many independent postprocess steps:

```text
               Transcript
                    │
        ┌───────────┼────────────┐
        ▼           ▼            ▼
 postprocess   postprocess   postprocess
   summary       tasks          jira
        │           │             │
        ▼           ▼             ▼
 summary.md    tasks.json    jira.csv
```

That fan-out is exactly why [`vd-pipeline`](../vd-pipeline/) is a **DAG** Executor — independent `postprocess` steps share inputs and run concurrently when ready.

---

## Why recipes, not built-in modes

Without this rule the tool becomes a catalog of product features. With recipes it stays a **capability**.

```bash
# invalid — nothing to run
vd-postprocess run --input meeting=meeting.json
# → error: no recipes specified
```

```bash
vd-postprocess run \
  --input meeting=meeting.json \
  --input context=.voxdecoder \
  --recipe ./summary.yaml \
  --recipe ./tasks.yaml
```

Same binary for every company:

| Company | Recipes they ship |
|---------|-------------------|
| A | `meeting-summary` |
| B | `engineering-rfc` |
| C | `medical-report` |

CLI unchanged. Only the recipe pack changes.

---

## Capability: `postprocess`

| `use` | Responsibility |
|-------|----------------|
| `preprocess` | Media → prepared media via **filter chain** |
| `transcribe` | Get text from audio/video |
| `prepare-context` | Get project knowledge |
| `fix-*` | Improve text |
| `diarize` | Speaker timeline |
| `meeting-merge` | Combine meeting artifacts |
| **`postprocess`** | Produce **new** artifacts from existing ones via **user recipe graphs** |

Parallel Job sketch:

```yaml
steps:
  - use: fix-terms
    id: transcript
    # …

  - use: postprocess
    id: summary
    inputs:
      transcript:
        artifact: transcript
      meeting:
        artifact: meeting
    options:
      recipes: [./summary.yaml]
      # runner override optional — recipe already has a default
      variables: { audience: Executives }

  - use: postprocess
    id: tasks
    inputs:
      transcript:
        artifact: transcript
      meeting:
        artifact: meeting
    options:
      recipes: [./tasks.yaml]

  - use: postprocess
    id: jira
    inputs:
      meeting:
        artifact: meeting
      glossary:
        artifact: terms.yml
    options:
      recipes: [./jira.yaml]
```

No special Executor rules — same DAG scheduling as every other capability.

One step may list **several recipes**; fan-out across **steps** is preferred when recipes are independent (clearer ids, better parallelism).

---

## Pipeline placement

Linear tail is fine when you want it:

```text
transcribe → prepare-context → fix-* → postprocess
```

Official shape that motivates the DAG:

```text
                 ┌──► postprocess(summary) ──► summary.md
Transcript ──────┼──► postprocess(tasks)   ──► tasks.json
                 └──► postprocess(jira)    ──► jira.csv
```

Meeting Jobs may attach postprocess after `meeting-merge`, or in parallel with other branches — planner’s choice. `vd-meeting` does not hard-code summary/tasks.

---

## Platform model

VoxDecoder’s process layer is **three complementary executors**, each at its own abstraction level:

```text
                 Media
                   │
                   ▼
           vd-preprocess
             (Filter Graph)
                   │
                   ▼
              Artifacts
                   │
                   ▼
             vd-pipeline
             (Capability DAG)
                   │
                   ▼
              Artifacts
                   │
                   ▼
          vd-postprocess
            (Recipe Graph)
                   │
                   ▼
          Derived Artifacts
```

| Level | What it executes |
|-------|------------------|
| **`vd-preprocess`** | Graph of media filters (`ffmpeg`, `deepfilternet`, …) over media |
| **`vd-pipeline`** | DAG of capabilities (`transcribe`, `diarize`, `meeting-merge`, `postprocess`, …) |
| **`vd-postprocess`** | Graph of recipe nodes (`LLM`, `process`, `http`, `mcp`, …) over artifacts |

| Layer | Role |
|-------|------|
| **Builders** | `vd-pipeline` CLI, `vd-meeting`, MCP, `vd-srv` |
| **Capabilities** | `preprocess`, `transcribe`, `prepare-context`, `fix-*`, `diarize`, `meeting-merge`, `postprocess` |
| **Job Executor** | One runtime for CLI / MCP / `vd-srv` (capability DAG) |
| **Artifacts** | Only data exchange between steps |

Twin leaf abstractions:

| Tool | Unit of work | Who runs a step |
|------|--------------|-----------------|
| [`vd-preprocess`](../vd-preprocess/) | **Filter** in a graph | media **provider** |
| **`vd-postprocess`** | **Node** in a recipe graph | **`ExecutionRunner`** |

New product features later = new capability and/or artifact type — not a fourth unrelated executor.

---

## Boundaries

| Tool | Owns |
|------|------|
| [`vd-pipeline`](../vd-pipeline/) | Job DAG + Executor; binds `postprocess` |
| **`vd-postprocess`** | Recipe load · graph plan · runner invoke · write / register derived artifacts |
| User / company | Recipe packs (portable; own default runners) |
| MCP | Job JSON only |

`vd-postprocess` never:

- ships built-in summary/tasks/jira recipes
- invents recipes when none were given
- assumes every runner is an LLM
- puts the default runner only on the Job (recipe must be portable)
- owns Meeting Model or diarization
- replaces fix-*

---

## Guarantees

1. **No recipes → error** (exit 2), not a silent default pack.
2. **CLI ≡ capability** — same options in flags and Job `options`.
3. **Runner resolution: `CLI > Job > Config > Recipe`** (same wording everywhere).
4. **Recipe is an execution graph**; every node has a Runner (explicit or inherited); **no `needs` → parallel**.
5. **Inputs are unified InputRef** (`artifact` \| `from`); **outputs** declare `artifact` + `type` (+ optional `path`).
6. **`variables` ≠ `secrets`**; dry-run emits first-class **ExecutionPlan** — no invoke.

---

## Status note

v0 ships with `stub` runner for CI and a flatter recipe shape (`prompt` + `provider` hints). Target contract: portable recipe graph, `ExecutionRunner`, unified InputRef, `artifact`+`type` outputs, `variables`/`secrets`, first-class **ExecutionPlan**, `foreach` reserved. Runner priority always **`CLI > Job > Config > Recipe`**. Platform concept: three executors (Filter Graph / Capability DAG / Recipe Graph) — see root [README](../../../../README.md).
