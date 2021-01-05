# Contribution

Please use pull-requests and write tests for your changeset.

## Release

This process is not automated at the moment:

- bump the version in `Cargo.toml`
- run `make release`
    - needs authenticated `cargo`, `docker` and `gh`
    - will publish to crates.io, dockerhub and github
- update version in [hendrikmaus/homebrew-tap](https://github.com/hendrikmaus/homebrew-tap)
