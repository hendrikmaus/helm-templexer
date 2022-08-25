FROM mcr.microsoft.com/vscode/devcontainers/base:debian

# Install additional OS packages.
ARG DEBIAN_FRONTEND=noninteractive
RUN apt update && apt install -y \
  gcc \
  gcc-10-multilib \
  gcc-aarch64-linux-gnu \
  gcc-x86-64-linux-gnu

# Install Rust and Linux targets for the vscode user.
USER vscode
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ARG PATH=/home/vscode/.cargo/bin/:$PATH
RUN rustup target add \
  aarch64-unknown-linux-gnu \
  x86_64-unknown-linux-gnu
