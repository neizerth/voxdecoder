# VoxDecoder Desktop

Tauri 2 + React + TypeScript client. Speaks the Runtime API (local UDS / pipe → `vd-srv`); does not embed capability binaries.

Scaffold only — Runtime wiring lands later.

## Stack

| Layer | Tech |
|-------|------|
| UI | React 19 + TypeScript + Vite |
| Shell | Tauri 2 (`src-tauri/`) |
| Identifier | `com.voxdecoder.desktop` |

Rust crate stays **outside** the repo Cargo workspace via `workspace.exclude` (`src/desktop/src-tauri`) so Tauri deps do not pollute CLI builds. Build from this directory.

## Setup

```bash
cd src/desktop
npm install
```

System deps: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (Xcode CLT on macOS, WebView2 on Windows).

## Dev

```bash
npm run tauri:dev
```

Frontend-only (no native shell):

```bash
npm run dev
```

## Build

```bash
npm run tauri:build
```

## Layout

```text
src/desktop/
  src/           # React UI
  src-tauri/     # Rust host + tauri.conf.json
  package.json
```
