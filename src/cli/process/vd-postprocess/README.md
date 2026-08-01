# vd-postprocess — recipe executor for derived artifacts

Layout: [STRUCTURE.md](STRUCTURE.md).  
CLI surface: [cli.md](cli.md).  
Stack overview: [../README.md](../README.md) · [vd-pipeline](../vd-pipeline/) · [vd-meeting](../vd-meeting/).  
Shared crates (planned): [`vd-artifact`](../../../crates/vd-artifact/), [`vd-progress`](../../../crates/vd-progress/).  
Rust gates: [RUST.md](RUST.md).

**Status: implemented.** Workspace member: `src/cli/process/vd-postprocess`. Default provider: `stub` (CI / dry pipelines). OpenAI / Ollama / process / HTTP / MCP: typed but not wired yet.

## Core rule

```text
vd-postprocess is a universal recipe executor.

Artifact(s) + Recipe + Provider → Derived Artifacts.

It has no built-in recipes.
Without recipes, it does nothing (and errors).
```

> **vd-postprocess produces new artifacts from existing ones by user recipes.**  
> The CLI does not know about “summary”, “tasks”, “jira”, or “decisions”. Those are recipe files the user (or company) owns. One binary; many corporate packs.  
> **Provider** means *execution provider* — LLM, local process, script, HTTP, MCP tool — not “chat API only”.

## Contract

```text
Artifact(s)
      +
Recipe
      +
Provider
      ↓
Derived Artifacts
```

| Surface | Role |
|---------|------|
| **CLI** (`vd-postprocess run`) | Human UX: named inputs + recipes + provider |
| **`use: postprocess`** | Same implementation, scheduled by [`vd-pipeline`](../vd-pipeline/) Executor |
| **MCP / `vd-srv`** | Submit a Job step; never own provider SDKs |

`vd-postprocess` knows nothing about meetings, ASR engines, or speaker identity. Planners only add Job step(s) with `options.recipes` + `options.provider` (+ optional `variables`).

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

Recipes declare **outputs** (ids + paths). The Executor registers them as artifacts like any other step — not opaque side files.

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

Terminology: **Recipe** is the product name (file may still be called a “template” casually). Avoid “Workflow” here — that word belongs to Job DAGs / planners.

---

## Multi-input

Many recipes need more than one artifact at once (transcript + meeting + glossary + docs). Inputs are a **named map**, not a single `-i`:

```yaml
# Job step
- use: postprocess
  id: summary
  inputs:
    transcript: transcript      # artifact id from earlier step
    meeting: meeting
    context: .voxdecoder
    glossary: terms.yml
  options:
    recipes:
      - ./summary.yaml
    provider:
      type: openai
      model: gpt-5
    variables:
      language: Russian
      audience: Executives
```

CLI sugar:

```bash
--input transcript=out.txt \
--input meeting=meeting.json \
--input glossary=terms.yml
```

Primary path sugar (`-i FILE`) may map to a default binding name when only one input is needed — still optional convenience, not the model.

---

## Recipes belong to the user

The tool never embeds domain prompts or scripts. Recipes are files:

```yaml
recipes:
  - ./summary.yaml
  - ./tasks.yaml
  - ./jira.yaml
```

or

```bash
--recipe summary.yaml
--recipe tasks.yaml
```

A recipe declares **inputs it expects**, **outputs it produces** (with ids), optional **variables**, **output schema**, and how the **provider** should run (prompt body, command, HTTP payload — depending on provider type). Exact schema: [cli.md](cli.md#recipe-document).

Outputs are first-class artifacts, e.g.:

```text
summary.md
tasks.json
jira.csv
slides.md
minutes.docx
```

All register the same way in the Job (`outputs` / step `id`).

---

## Provider = execution provider

Not “LLM provider”. Anything that can **execute a recipe**:

| `provider.type` (examples) | Role |
|----------------------------|------|
| `openai` / `anthropic` / `ollama` / `gigachat` | Remote or local chat models |
| `process` / `python` | Local executable / script |
| `http` | HTTP endpoint |
| `mcp` | MCP tool call |

```yaml
provider:
  type: ollama
  model: qwen3
```

```yaml
provider:
  type: process
  command: ./bin/my-renderer
```

Auth / base URL / API keys via env + config — never baked into Meeting Model. MCP picks `provider` in Job JSON; it does not implement clients.

---

## Capability: `postprocess`

| `use` | Responsibility |
|-------|----------------|
| `transcribe` | Get text from audio/video |
| `prepare-context` | Get project knowledge |
| `fix-*` | Improve text |
| `diarize` | Speaker timeline |
| `meeting-merge` | Combine meeting artifacts |
| **`postprocess`** | Produce **new** artifacts from existing ones via **user recipes** |

Parallel Job sketch:

```yaml
steps:
  - use: fix-terms
    id: transcript
    # …

  - use: postprocess
    id: summary
    inputs: { transcript: transcript, meeting: meeting }
    options:
      recipes: [./summary.yaml]
      provider: { type: openai, model: gpt-5 }
      variables: { audience: Executives }

  - use: postprocess
    id: tasks
    inputs: { transcript: transcript, meeting: meeting }
    options:
      recipes: [./tasks.yaml]
      provider: { type: openai, model: gpt-5 }

  - use: postprocess
    id: jira
    inputs: { meeting: meeting, glossary: terms.yml }
    options:
      recipes: [./jira.yaml]
      provider: { type: process, command: ./tools/jira-export }
```

No special Executor rules — same DAG scheduling as every other capability.

One step may also list **several recipes** and emit several outputs in one invoke; fan-out across **steps** is preferred when recipes are independent (clearer ids, better parallelism).

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

## Platform model (after this capability)

| Layer | Role |
|-------|------|
| **Builders** | `vd-pipeline` CLI, `vd-meeting`, MCP, `vd-srv` |
| **Capabilities** | `transcribe`, `prepare-context`, `fix-*`, `diarize`, `meeting-merge`, `postprocess` |
| **Executor** | One runtime for CLI / MCP / `vd-srv` |
| **Artifacts** | Only data exchange between steps |

New product features later = new capability and/or artifact type — not a new Executor.

---

## Boundaries

| Tool | Owns |
|------|------|
| [`vd-pipeline`](../vd-pipeline/) | Job DAG + Executor; binds `postprocess` |
| **`vd-postprocess`** | Recipe load · provider invoke · write / register derived artifacts |
| User / company | Recipe packs |
| MCP | Job JSON only |

`vd-postprocess` never:

- ships built-in summary/tasks/jira recipes
- invents recipes when none were given
- assumes every provider is an LLM
- owns Meeting Model or diarization
- replaces fix-* 

---

## Guarantees

1. **No recipes → error** (exit 2), not a silent default pack.
2. **CLI ≡ capability** — same options in flags and Job `options`.
3. **Provider is an ExecutionProvider** under `provider.type` (+ type-specific fields).
4. **Recipes declare outputs** (ids + paths); one recipe → many `DerivedArtifact`s.
5. **Inputs are artifacts** (named `ArtifactRef` map); **variables** + optional **schema/mime**.
6. **Dry-run emits ExecutionPlan** after provider resolve — no invoke.

---

## Status note

Implemented with `stub` provider for CI. Other `ExecutionProvider` backends land without changing the Job / recipe contract.
