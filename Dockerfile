# Multi-stage Dockerfile for txtcode
#
# Build:  docker build -t txtcode/txtcode:latest .
# Run:    docker run --rm txtcode/txtcode:latest script.tc
# REPL:   docker run -it --rm txtcode/txtcode:latest

# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.80-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches

# Build release binary (statically linked via musl)
RUN RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build --release --bin txtcode \
    && strip target/release/txtcode

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

# Non-root user for security
RUN addgroup -S txtcode && adduser -S txtcode -G txtcode

COPY --from=builder /src/target/release/txtcode /usr/local/bin/txtcode

# Copy stdlib packages so offline installs work
COPY packages /home/txtcode/.txtcode-env/default/packages
COPY registry /home/txtcode/.txtcode-registry

RUN chown -R txtcode:txtcode /home/txtcode

USER txtcode
WORKDIR /workspace

ENTRYPOINT ["txtcode"]
CMD ["--help"]

# Labels
LABEL org.opencontainers.image.title="Txtcode"
LABEL org.opencontainers.image.description="Txtcode Programming Language Runtime"
LABEL org.opencontainers.image.url="https://txtcode.dev"
LABEL org.opencontainers.image.source="https://github.com/txtcode/txtcode"
LABEL org.opencontainers.image.licenses="MIT"
