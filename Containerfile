FROM rustlang/rust:nightly AS chef

RUN rustup override set nightly-2024-08-21
RUN apt-get update && apt-get install protobuf-compiler -y

RUN cargo install cargo-chef
WORKDIR /app/argocd-lint

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/argocd-lint/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

FROM debian:stable-slim AS runtime
WORKDIR /app/argocd-lint

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

EXPOSE 8080

COPY --from=builder /app/argocd-lint/target/release/argocd-lint .
COPY ./config.yaml /app/argocd-lint/config.yaml

LABEL org.opencontainers.image.source = "https://github.com/cfi2017/argocd-lint"
CMD ["/app/argocd-lint/argocd-lint"]