#!/usr/bin/env bash
# https://github.com/yoshihitoh/refmt
refmt -i config_example.toml --input-format toml -o config_example.yaml --output-format yaml
refmt -i config_example.toml --input-format toml -o config_example.json --output-format json
