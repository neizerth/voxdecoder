# InputSource (shared)

**Status:** living note  
**Used by:** Runtime API clients — MCP, Desktop, CLI, REST, Web — and Domain Requests.

An **InputSource** is how a client points the Runtime at media or prior artifacts.  
Same shape everywhere; the gateway does not invent a private upload protocol.

```text
InputSource

  path       filesystem path visible to the Runtime / host
  uri        file:// · https:// · … (Runtime policy decides allowed schemes)
  artifact   ArtifactId already known to the Runtime
  blob       small inline payload (e.g. MCP binary / base64) — Runtime stores it
```

## Examples

```yaml
audio:
  path: /work/meeting.wav

audio:
  uri: file:///work/meeting.wav

audio:
  artifact: art_01H…

audio:
  blob: …   # small files only; large media should use path / uri / artifact
```

## Rules

* Resolution and persistence belong to the **Runtime** (or host mounts), not to individual frontends.
* `blob` is for small payloads; do not treat MCP as a bulk file store.
* Domain Requests (`AudioRequest`, `MeetingRequest`, …) and Jobs reuse this type.

Related: [`docs/runtime.md`](runtime.md) · [`vd-mcp`](../src/cli/manage/vd-mcp/) · [`vd-srv`](../src/cli/manage/vd-srv/).
