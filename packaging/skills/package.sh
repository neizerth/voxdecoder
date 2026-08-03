#!/usr/bin/env bash
# Skill release packaging (ADR 0009).
#
# Does NOT install into AI applications — that is vdctl's job (ADR 0005).
#
# Usage:
#   ./packaging/skills/package.sh build     # stage package trees under dist/skills/staging
#   ./packaging/skills/package.sh package   # build + emit ZIP + DXT under dist/skills
#   ./packaging/skills/package.sh clean
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

skills_src="$root/skills"
packaging="$root/packaging/skills"
out="$root/dist/skills"
staging="$out/staging"
manifest="$packaging/manifest.json"

die() { echo "error: $*" >&2; exit 1; }

# Claude / Agent Skills require SKILL.md + YAML frontmatter (name, description).
# Repo source stays skills/<id>/skill.md for vdctl (ADR 0005); packaging normalizes.
DESC_MAX=200

list_skills() {
  local d id
  for d in "$skills_src"/*/skill.md; do
    [[ -f "$d" ]] || continue
    id="$(basename "$(dirname "$d")")"
    # Skip template / non-skill dirs
    [[ "$id" == "TEMPLATE" ]] && continue
    printf '%s\n' "$id"
  done | sort
}

# First paragraph under ## Purpose (fallback: first non-heading line after H1).
extract_description() {
  local file="$1" desc
  desc="$(
    awk '
      /^##[[:space:]]+[Pp]urpose([[:space:]]|$)/ { p=1; next }
      p && /^##[[:space:]]/ { exit }
      p && /^#/ { exit }
      p && NF { print; exit }
    ' "$file"
  )"
  if [[ -z "$desc" ]]; then
    desc="$(
      awk '
        /^#[[:space:]]/ && !t { t=1; next }
        t && /^#/ { exit }
        t && NF { print; exit }
      ' "$file"
    )"
  fi
  printf '%s' "$desc"
}

yaml_double_quote() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/}"
  printf '"%s"' "$s"
}

truncate_chars() {
  local s="$1" max="$2"
  if ((${#s} <= max)); then
    printf '%s' "$s"
    return
  fi
  # Prefer a clean cut at a word boundary when possible.
  local cut="${s:0:max}"
  if [[ "$cut" == *" "* ]]; then
    cut="${cut% *}"
  fi
  printf '%s…' "$cut"
}

# Drop leading YAML frontmatter if present (idempotent re-stage).
body_without_frontmatter() {
  local file="$1"
  awk '
    NR==1 && $0=="---" { fm=1; next }
    fm && $0=="---" { fm=0; next }
    fm { next }
    { print }
  ' "$file"
}

# Emit Claude-compatible SKILL.md from repo skill.md.
write_skill_md() {
  local id="$1" src="$2" dest="$3"
  local desc
  desc="$(extract_description "$src")"
  [[ -n "$desc" ]] || desc="VoxDecoder skill ${id}"
  desc="$(truncate_chars "$desc" "$DESC_MAX")"

  {
    printf '%s\n' '---'
    printf 'name: %s\n' "$id"
    printf 'description: %s\n' "$(yaml_double_quote "$desc")"
    printf '%s\n\n' '---'
    body_without_frontmatter "$src"
  } >"$dest"
}

cmd_clean() {
  rm -rf "$out"
  echo "cleaned $out"
}

cmd_build() {
  [[ -d "$skills_src" ]] || die "skills/ not found"
  [[ -f "$manifest" ]] || die "missing $manifest"

  rm -rf "$staging"
  mkdir -p "$staging"

  local id count=0 src
  while IFS= read -r id; do
    [[ -n "$id" ]] || continue
    src="$skills_src/$id/skill.md"
    mkdir -p "$staging/$id"
    # Distribution layout: SKILL.md + frontmatter (Claude / Agent Skills).
    write_skill_md "$id" "$src" "$staging/$id/SKILL.md"
    [[ -f "$skills_src/$id/README.md" ]] && cp "$skills_src/$id/README.md" "$staging/$id/README.md"
    [[ -d "$skills_src/$id/examples" ]] && cp -R "$skills_src/$id/examples" "$staging/$id/examples"
    count=$((count + 1))
    echo "  staged $id → SKILL.md"
  done < <(list_skills)

  [[ "$count" -gt 0 ]] || die "no skills found under skills/*/skill.md"
  echo "build:skills → $staging ($count skills)"
}

make_zip() {
  local src_dir="$1" archive="$2"
  mkdir -p "$(dirname "$archive")"
  rm -f "$archive"
  (
    cd "$(dirname "$src_dir")"
    zip -qr "$archive" "$(basename "$src_dir")"
  )
}

cmd_package() {
  cmd_build

  local id
  while IFS= read -r id; do
    [[ -n "$id" ]] || continue
    make_zip "$staging/$id" "$out/$id.zip"
    echo "  wrote $out/$id.zip"
  done < <(list_skills)

  # Skill pack (.dxt): zip of manifest + skills/ tree.
  # Distinct from local gateway Bundle (.mcpb) — see ADR 0009 naming note.
  local pack_stage="$out/pack"
  rm -rf "$pack_stage"
  mkdir -p "$pack_stage/skills"
  cp "$manifest" "$pack_stage/manifest.json"
  while IFS= read -r id; do
    [[ -n "$id" ]] || continue
    cp -R "$staging/$id" "$pack_stage/skills/$id"
  done < <(list_skills)

  local dxt="$out/voxdecoder-skills.dxt"
  rm -f "$dxt"
  (
    cd "$pack_stage"
    zip -qr "$dxt" manifest.json skills
  )
  rm -rf "$pack_stage"
  echo "  wrote $dxt"

  echo "package:skills → $out"
  ls -1 "$out"/*.zip "$out"/*.dxt 2>/dev/null || true
}

usage() {
  cat <<'EOF'
Usage: packaging/skills/package.sh <build|package|clean>

  build     Stage Skill trees under dist/skills/staging (no archives).
  package   Build + emit per-Skill ZIP and voxdecoder-skills.dxt.
  clean     Remove dist/skills.

Never installs into AI applications (use vdctl mcp update).
EOF
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    build) cmd_build ;;
    package) cmd_package ;;
    clean) cmd_clean ;;
    -h|--help|help|"") usage; [[ -n "$cmd" ]] || exit 1 ;;
    *) die "unknown command: $cmd (try build|package|clean)" ;;
  esac
}

main "$@"
