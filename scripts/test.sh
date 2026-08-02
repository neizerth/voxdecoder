#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

run_all() {
  cargo test -p vd-gigaam
  cargo test -p vd-artifact -p vd-output -p vd-progress
  cargo test -p vd-assets
  cargo test -p vd-diarize
    cargo test -p vd-meeting
  cargo test -p vd-postprocess
  cargo test -p vd-pipeline
  cargo test -p vd-srv
  cargo test -p vd-fix-casing
  cargo test -p vd-fix-asr
  cargo test -p vd-fix-terms
}

case "${1:-all}" in
  all)
    run_all
    ;;
  vd-gigaam)
    cargo test -p vd-gigaam "${@:2}"
    ;;
  crates)
    cargo test -p vd-artifact -p vd-output -p vd-progress "${@:2}"
    ;;
  vd-assets)
    cargo test -p vd-assets "${@:2}"
    ;;
  vd-diarize)
    cargo test -p vd-diarize "${@:2}"
    ;;
  vd-meeting)
    cargo test -p vd-meeting "${@:2}"
    ;;
  vd-postprocess)
    cargo test -p vd-postprocess "${@:2}"
    ;;
  vd-pipeline)
    cargo test -p vd-pipeline "${@:2}"
    ;;
  vd-srv)
    cargo test -p vd-srv "${@:2}"
    ;;
  vd-fix-casing)
    cargo test -p vd-fix-casing "${@:2}"
    ;;
  vd-fix-asr)
    cargo test -p vd-fix-asr "${@:2}"
    ;;
  vd-fix-terms)
    cargo test -p vd-fix-terms "${@:2}"
    ;;
  *)
    echo "usage: $0 [all|vd-gigaam|crates|vd-assets|vd-diarize|vd-meeting|vd-postprocess|vd-pipeline|vd-srv|vd-fix-casing|vd-fix-asr|vd-fix-terms] [cargo test args...]" >&2
    exit 2
    ;;
esac
