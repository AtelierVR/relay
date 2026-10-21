# Stage 1: Build
# Pin the same toolchain as CI (`dtolnay/rust-toolchain@stable`): several locked crates
# already require >= 1.89 (e.g. uuid 1.28), so 1.88 no longer compiles.
FROM rust:1.99-alpine AS build
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static
WORKDIR /app

# Copier les manifests et créer des dummies pour le cache
COPY Cargo.toml ./
RUN mkdir src && echo "fn main(){}" > src/main.rs

# Build des dépendances (layer avec cache)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    rm -rf src

# Copier le vrai code source
COPY src ./src

# Build final (réutilise les dépendances compilées)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --target-dir /app/target --release && \
    cp /app/target/release/noxrelay /app/noxrelay

# Stage 2: Runtime
FROM alpine:3.21 AS runtime
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=build /app/noxrelay ./noxrelay
EXPOSE 23032/udp
ENTRYPOINT ["./noxrelay"]
