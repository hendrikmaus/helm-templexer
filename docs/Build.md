# Building From Source

Install the linux musl target:

```shell
rustup target add x86_64-unknown-linux-musl
```

Build the release binary:

```shell
cargo build --release --target x86_64-unknown-linux-musl
```

Build the linux container:

```shell
docker build -t <tag> .
```
