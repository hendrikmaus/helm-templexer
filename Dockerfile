FROM lukemathwalker/cargo-chef as planner
WORKDIR app
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM lukemathwalker/cargo-chef as cacher
WORKDIR app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust as builder
WORKDIR app
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release --bin helm-templexer

FROM debian:buster-slim
COPY --from=builder /app/target/release/helm-templexer /usr/local/bin

RUN apt-get update && apt-get install -y curl

ARG HELM_VERSION=3.5.4
ARG HELM_SHASUM=a8ddb4e30435b5fd45308ecce5eaad676d64a5de9c89660b56face3fe990b318
RUN curl --location --retry 3 --show-error --silent -O "https://get.helm.sh/helm-v${HELM_VERSION}-linux-amd64.tar.gz" \
 && echo "${HELM_SHASUM} helm-v${HELM_VERSION}-linux-amd64.tar.gz" | sha256sum --check --strict --status \
 && tar -xzf helm-v${HELM_VERSION}-linux-amd64.tar.gz \
 && rm helm-v${HELM_VERSION}-linux-amd64.tar.gz \
 && mv "linux-amd64/helm" /usr/bin/ \
 && chmod +x /usr/bin/helm

CMD ["--help"]
ENTRYPOINT ["helm-templexer"]
