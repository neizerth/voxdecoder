#!/bin/sh
# Placeholder until `vd-mcp` is implemented (see src/cli/manage/vd-mcp/README.md).
# MCP talks to the Runtime over Transport — not HTTP.
echo "vd-mcp: not implemented yet — image reserved for the MCP interface role" >&2
echo "Runtime Transport: ${VD_TRANSPORT:-tcp} ${VD_TCP:-${VD_SOCKET:-}}" >&2
exit 1
