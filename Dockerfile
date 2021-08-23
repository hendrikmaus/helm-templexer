FROM debian:buster-slim as builder
SHELL ["/bin/bash", "-ceuxo", "pipefail"]
RUN apt-get update && apt-get install -y curl

ARG HELM_VERSION=3.5.4
ARG HELM_SHASUM=a8ddb4e30435b5fd45308ecce5eaad676d64a5de9c89660b56face3fe990b318
RUN curl --location --retry 3 --show-error --silent -O "https://get.helm.sh/helm-v${HELM_VERSION}-linux-amd64.tar.gz" \
 && echo "${HELM_SHASUM} helm-v${HELM_VERSION}-linux-amd64.tar.gz" | sha256sum --check --strict --status \
 && tar -xzf helm-v${HELM_VERSION}-linux-amd64.tar.gz \
 && rm helm-v${HELM_VERSION}-linux-amd64.tar.gz \
 && mv "linux-amd64/helm" /usr/bin/ \
 && chmod +x /usr/bin/helm

FROM debian:buster-slim as runtime
# labels according to opencontainers https://github.com/opencontainers/image-spec/blob/main/annotations.md
LABEL org.opencontainers.image.authors="Hendrik Maus <aidentailor@gmail.com>"
LABEL org.opencontainers.image.url="https://github.com/hendrikmaus/helm-templexer"
LABEL org.opencontainers.image.documentation="https://github.com/hendrikmaus/helm-templexer"
LABEL org.opencontainers.image.source="https://github.com/hendrikmaus/helm-templexer/blob/master/Dockerfile"
LABEL org.opencontainers.image.description="Render Helm charts for multiple environments using explicit configuration."
COPY --from=builder /usr/bin/helm /usr/bin/
COPY target/x86_64-unknown-linux-musl/release/helm-templexer /usr/bin/
USER 1001
CMD ["--help"]
ENTRYPOINT ["/usr/bin/helm-templexer"]
