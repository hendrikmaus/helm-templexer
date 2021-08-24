#!/usr/bin/env bash

refmt -i config_example.toml --input-format toml -o config_example.yaml --output-format yaml
refmt -i config_example.toml --input-format toml -o config_example.json --output-format json
