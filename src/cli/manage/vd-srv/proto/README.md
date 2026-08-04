# Runtime gRPC proto (`voxdecoder.runtime.v1`)

IDL: [`runtime/v1/runtime.proto`](runtime/v1/runtime.proto).

## Codegen

**Rust** — `tonic-build` inside `vd-srv` (`cargo build -p vd-srv`).

**TypeScript** (Apollo-style: schema → one command → client file):

```bash
cd src/desktop
npm run generate
# → grpc-client/src/gen/ (gitignored; also via grpc-client postinstall)
```

First time: `cd grpc-client && npm install`. Details: [`grpc-client/README.md`](../../../../desktop/grpc-client/README.md).

## Typed vs JsonBody

| Surface | Shape |
|---------|--------|
| `GetJob` / `ListJobs` / `CancelJob` | `JobView` |
| `WatchJob` | `stream Event` |
| `Health` / `Ready` | `HealthResponse` |
| `SubmitJob` / `Plan*` | `JsonBody` |
| `Live` / `Doctor` / `ServerInfo` | `JsonBody` |

```bash
vd-srv serve --grpc 127.0.0.1:7702
```
