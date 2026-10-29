# syntax=docker/dockerfile:1
#
# Crate Tab – Die Getränkekasse für euren Verein, geschrieben in Rust 🦀
#
# Mehrstufiger Build:
#   1. „builder“ kompiliert die Binärdatei. Er läuft immer auf der Architektur
#      des Build-Rechners und cross-kompiliert bei Bedarf für arm64 (Raspberry Pi).
#      Das ist viel schneller als Emulation mit QEMU.
#   2. Das Laufzeit-Image enthält nur die Binärdatei (distroless, ~30 MB).
#
# Multi-Arch-Build:
#   docker buildx build --platform linux/amd64,linux/arm64 -t ghcr.io/dertika/cratetab .

ARG RUST_VERSION=1

FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION}-bookworm AS builder
ARG TARGETARCH
WORKDIR /src

# Cross-Compiler für arm64 nur installieren, wenn er gebraucht wird
RUN if [ "$TARGETARCH" = "arm64" ] && [ "$(dpkg --print-architecture)" != "arm64" ]; then \
      apt-get update && apt-get install -y --no-install-recommends gcc-aarch64-linux-gnu libc6-dev-arm64-cross \
      && rm -rf /var/lib/apt/lists/*; \
    fi

# Rust-Ziel passend zur Plattform wählen
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-gnu > /rust-target ;; \
      arm64) echo aarch64-unknown-linux-gnu > /rust-target ;; \
      *) echo "Nicht unterstützte Architektur: $TARGETARCH" && exit 1 ;; \
    esac \
    && rustup target add "$(cat /rust-target)"

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY locales ./locales
COPY assets ./assets

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked,id=cratetab-target-${TARGETARCH} \
    cargo build --release --locked --target "$(cat /rust-target)" \
    && cp "target/$(cat /rust-target)/release/cratetab" /cratetab

# Datenverzeichnis vorbereiten (gehört dem unprivilegierten Benutzer 65532)
RUN mkdir -p /data && chown 65532:65532 /data

FROM gcr.io/distroless/cc-debian12:nonroot

LABEL org.opencontainers.image.title="Crate Tab" \
      org.opencontainers.image.description="Die Getränkekasse für euren Verein – geschrieben in Rust 🦀" \
      org.opencontainers.image.source="https://github.com/dertika/cratetab"

COPY --from=builder /cratetab /usr/local/bin/cratetab
COPY --from=builder --chown=65532:65532 /data /data

ENV CRATETAB_DATABASE=/data/cratetab.db \
    CRATETAB_LISTEN=0.0.0.0:8080

USER 65532:65532
VOLUME ["/data"]
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD ["/usr/local/bin/cratetab", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/cratetab"]
CMD ["serve"]
