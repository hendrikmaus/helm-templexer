# ⎈ Helm Templexer

Render Helm charts for multiple environments with _explicit config_ while keeping the overhead at ease.

```shell
cat > my-app.yaml <<EOF
version: v2
chart: tests/data/nginx-chart
release_name: my-app
output_path: manifests
deployments:
  - name: edge
EOF

helm-templexer render my-app.yaml
```

Outcome:

```text
❯ exa -TL3 manifests
manifests
└── edge
   └── my-app
      └── nginx-chart
```

## Configuration

Configuration can be provided as YAML format.

> All paths are evaluated relative to the configuration ile during execution.

| **Parameter**        | **Description**                                                                                                                                                                                                                                                                    | **Condition** | **Default** | **Example**                          |
|----------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-------------:|-------------|--------------------------------------|
| `version`            | Schema version to use                                                                                                                                                                                                                                                              |  **required** |             | `"v1"`                               |
| `helm_version`       | SemVer version constraint to require                                                                                                                                                                                                                                               |    optional   | `~3`        |                                      |
| `enabled`            | Whether to render deployments or not                                                                                                                                                                                                                                               |    optional   | `true`      |                                      |
| `chart`              | Path to the chart to render                                                                                                                                                                                                                                                        |  **required** |             | `"path/to/some-chart"`               |
| `namespace`          | Namespace to pass on to `helm`; when omitted, no namespace is passed                                                                                                                                                                                                               |    optional   | `""`        |                                      |
| `release_name`       | Release name to pass to `helm`                                                                                                                                                                                                                                                     |  **required** |             | `"some-release"`                     |
| `output_path`        | Base path to use for writing the manifests to disk.<br><br>The fully-qualified output path is built as follows (`config` refers to the top-level):<br>`config.output_path/deployment.name/<[config/deployment].release_name>`                                                      |  **required** |             |                                      |
| `additional_options` | Pass additional options to `helm template`; you can use all supported options of the tool.<br><br>Common use case: use `--set-string` to provide a container tag to use.<br>This can be achieved by modifying the configuration file in your build pipeline using mikefarah/yq |    optional   | `[]`        | `["--set-string image.tag=42"]`      |
| `values`             | A list of base value files which are passed to each `helm template` call.<br>This is commonly used to provide a sane base config.                                                                                                                                                  |    optional   | `[]`        |                                      |
| `deployments`        | The list of deployments to render.                                                                                                                                                                                                                                                 |  **required** |             | `[[deployments]]`<br>`name = "edge"` |

Deployments can override several top-level fields:

| **Parameter**        | **Description**                                                    | **Condition** | **Default** | **Example**    |
|----------------------|--------------------------------------------------------------------|---------------|-------------|----------------|
| `name`               | Name of the deployment; only used in the output path               | **required**  |             | `"edge-eu-w4"` |
| `enabled`            | Allows for disabling individual deployments                        | optional      | `true`      |                |
| `release_name`       | Override the release name                                          | optional      | `""`        |                |
| `additional_options` | Additional options, as seen above, but specific to this deployment | optional      | `[]`        |                |
| `values`             | Value files to use for this deployment                             | optional      | `[]`        |                |

## Additional Options to The Render Command

Use `--additional-options` to pass data to the underlying `helm template` call. Beware that these additional options get added to *every* call, i.e. to each deployment.

A common use case we found was to provide the container tag:

```shell
helm-templexer render --additional-options="--set-string image.tag=${revision}" my-app.yaml
```

Use `--filter` to render a specific deployment. Example: To render only the `prod`, pass the regex to the filter option.

```shell
helm-templexer render --filter="prod" my-app.yaml
```

## Installation

### Docker

```shell
# create the directory where helm-templexer will render to
mkdir -p tests/data/manifests

# let helm-templexers user id (1001) own the directory
sudo chown -R 1001 tests/data/manifests

# pull and run the image
docker pull ghcr.io/hendrikmaus/helm-templexer
docker run --rm --volume $(pwd):/srv --workdir /srv/tests/data ghcr.io/hendrikmaus/helm-templexer render config_example.yaml 
```

Include `helm-templexer` in your `Dockerfile`:

```Dockerfile
FROM ghcr.io/hendrikmaus/helm-templexer AS helm-templexer-provider
COPY --from=helm-templexer-provider /usr/bin/helm-templexer /usr/bin
COPY --from=helm-templexer-provider /usr/bin/helm /usr/bin
```

### Homebrew

```shell
brew tap hendrikmaus/tap
brew install helm-templexer
```

### Cargo Install

Helm Templexer is written in [Rust](http://www.rust-lang.org/). You will need `rustc` version 1.35.0 or higher. The recommended way to install Rust is from the official download page. Once you have it set up, a simple `make install` will compile `helm-templexer` and install it into `$HOME/.cargo/bin`.

If you’re using a recent version of Cargo (0.5.0 or higher), you can use the `cargo install` command:

```shell
cargo install helm-templexer
```

Cargo will build the binary and place it in `$HOME/.cargo/bin` (this location can be overridden by setting the --root option).
