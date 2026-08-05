# VoxDecoder Desktop

Tauri 2 + React + TypeScript. Runtime API client — local `vd-srv` (gRPC / HTTP).

## Setup

```bash
cd src/desktop
npm install
cd grpc-client && npm install && cd ..   # postinstall → generate → src/gen/
```

## Generate gRPC client (schema → TS)

Schema: `vd-srv/proto`. Output `grpc-client/src/gen/` is **gitignored**.

```bash
npm run generate
npm run generate:verify   # optional smoke
```

## Dev

```bash
npm run tauri:dev
```

## Layout

```text
src/desktop/
  src/           # React UI
  src-tauri/     # Tauri host
  grpc-client/   # buf codegen (proto → src/gen/)
```
