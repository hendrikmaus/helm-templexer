FROM rust:slim-buster as builder
WORKDIR /usr/src/helm-templexer
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/helm-templexer /usr/local/bin/

CMD ["--help"]
ENTRYPOINT ["helm-templexer"]
