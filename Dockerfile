# Stage 1: Build
FROM rust:1.85-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Pre-fetch dependencies (layer cache)
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM alpine:3.21 AS runtime
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/nox-relay ./nox-relay
EXPOSE 23032/udp
ENTRYPOINT ["./nox-relay"]
