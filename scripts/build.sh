#!/usr/bin/env bash
# Build Runtime capability binaries (same package set as Docker `runtime`).
#
# Platform features (ADR 0002):
#   macOS  → vd-gigaam/metal (unless --cpu)
#   Linux / Docker / --cpu → no Metal
#   CUDA features → future (--cuda when wired)
#
# Usage:
#   ./scripts/build.sh              # --release + host defaults
#   ./scripts/build.sh --debug
#   ./scripts/build.sh --cpu        # force CPU (Docker / CI Linux)
#   ./scripts/build.sh -- <extra cargo args>
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

profile=(--release)
force_cpu=0
features=()
packages=(
  -p vdctl
  -p vd-srv
  -p vd-mcp
  -p vd-pipeline
  -p vd-meeting
  -p vd-preprocess
  -p vd-postprocess
  -p vd-url
  -p vd-assets
  -p vd-diarize
  -p vd-gigaam
  -p vd-fix-casing
  -p vd-fix-asr
  -p vd-fix-disfluency
  -p vd-fix-terms
  -p vd-fix-layout
  -p vd-fix-overlap
)

extra=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug | --dev)
      profile=()
      shift
      ;;
    --release)
      profile=(--release)
      shift
      ;;
    --cpu)
      force_cpu=1
      shift
      ;;
    --cuda)
      echo "build.sh: --cuda not wired yet (ADR 0002 future)" >&2
      exit 2
      ;;
    --)
      shift
      extra+=("$@")
      break
      ;;
    *)
      extra+=("$1")
      shift
      ;;
  esac
done

# Docker / CI can also set VD_BUILD_FORCE_CPU=1
if [[ "${VD_BUILD_FORCE_CPU:-}" == "1" ]]; then
  force_cpu=1
fi

os="$(uname -s 2>/dev/null || echo unknown)"
if [[ "$force_cpu" -eq 0 && "$os" == "Darwin" ]]; then
  features=(--features vd-gigaam/metal)
  echo "build.sh: macOS → enabling vd-gigaam/metal" >&2
else
  echo "build.sh: CPU build (no Metal features)" >&2
fi

cmd=(cargo build "${profile[@]}" "${packages[@]}")
if [[ ${#features[@]} -gt 0 ]]; then
  cmd+=("${features[@]}")
fi
if [[ ${#extra[@]} -gt 0 ]]; then
  cmd+=("${extra[@]}")
fi
exec "${cmd[@]}"
