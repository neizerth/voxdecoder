# VoxDecoder — one build, role-based images.
#
#   docker build -t voxdecoder/runtime --target runtime .
#   docker build -t voxdecoder/runtime --target runtime --build-arg WITH_FFMPEG=0 .
#   docker build -t voxdecoder/mcp --target mcp .
#
# Runtime PID 1 = `vd-srv serve`. Linux image only (ADR 0002 — no Metal in containers).
# See docs/runtime.md · docs/adr/0002-build-and-container-strategy.md

# syntax=docker/dockerfile:1

ARG WITH_FFMPEG=1

# ── build all capability binaries once ──────────────────────────────────────
FROM rust:bookworm AS builder

WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Linux: CPU Candle only (ADR 0002 — containers never ship Metal).
RUN chmod +x scripts/build.sh && ./scripts/build.sh --cpu

# ── shared runtime filesystem ───────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime-base

ARG WITH_FFMPEG=1

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libgomp1 \
    && if [ "$WITH_FFMPEG" = "1" ]; then \
         apt-get install -y --no-install-recommends ffmpeg; \
       fi \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Unified models root; per-tool subdirs (gigaam/, diarize/, …).
# Legacy tool-specific envs still work and default under this tree.
ENV PATH="/usr/local/bin:${PATH}" \
    VD_MODELS_DIR=/models \
    VD_GIGAAM_MODELS_DIR=/models/gigaam \
    VD_PROJECT_DIR=/data/project \
    VD_SRV_CONFIG=/etc/voxdecoder/runtime.toml \
    VD_SRV_DATA=/data/srv \
    RUST_LOG=info

COPY --from=builder /src/target/release/vd-srv /usr/local/bin/
COPY --from=builder /src/target/release/vd-pipeline /usr/local/bin/
COPY --from=builder /src/target/release/vd-meeting /usr/local/bin/
COPY --from=builder /src/target/release/vd-preprocess /usr/local/bin/
COPY --from=builder /src/target/release/vd-postprocess /usr/local/bin/
COPY --from=builder /src/target/release/vd-assets /usr/local/bin/
COPY --from=builder /src/target/release/vd-diarize /usr/local/bin/
COPY --from=builder /src/target/release/vd-gigaam /usr/local/bin/
COPY --from=builder /src/target/release/vd-fix-casing /usr/local/bin/
COPY --from=builder /src/target/release/vd-fix-asr /usr/local/bin/
COPY --from=builder /src/target/release/vd-fix-terms /usr/local/bin/

COPY docker/runtime.toml /etc/voxdecoder/runtime.toml

RUN mkdir -p \
        /data/srv \
        /data/cache \
        /data/jobs \
        /data/artifacts \
        /data/logs \
        /data/project \
        /models/gigaam \
        /models/diarize \
        /models/postprocess \
        /work \
    && useradd --system --uid 10001 --home /data --shell /usr/sbin/nologin vox \
    && chown -R vox:vox /data /models /work /etc/voxdecoder

USER vox
VOLUME ["/data", "/models", "/work"]

# ── image: voxdecoder/runtime ───────────────────────────────────────────────
FROM runtime-base AS runtime

EXPOSE 7701

# Fixed entry; network/data defaults live in CMD (overridable).
ENTRYPOINT ["vd-srv", "serve"]
CMD ["--transport", "tcp", "--tcp", "0.0.0.0:7701", "--data-dir", "/data/srv"]

# Client tools read VD_SRV_CONFIG (tcp → 127.0.0.1 for health); serve binds 0.0.0.0 via CMD.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD vd-srv ping || exit 1

# ── image: voxdecoder/mcp (interface only; no GPU / no Executor) ────────────
FROM debian:bookworm-slim AS mcp

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /data --shell /usr/sbin/nologin vox

WORKDIR /app
COPY docker/vd-mcp-stub.sh /usr/local/bin/vd-mcp
RUN chmod +x /usr/local/bin/vd-mcp

USER vox
# Same Transport knobs as vd-srv clients — not HTTP.
ENV VD_TRANSPORT=tcp \
    VD_TCP=runtime:7701
ENTRYPOINT ["vd-mcp"]
CMD []
