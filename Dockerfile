FROM rust:slim-buster as builder
WORKDIR /usr/src/

RUN USER=root cargo new helm-templexer
WORKDIR /usr/src/helm-templexer
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/helm-templexer /usr/local/bin/

CMD ["--help"]
ENTRYPOINT ["helm-templexer"]
