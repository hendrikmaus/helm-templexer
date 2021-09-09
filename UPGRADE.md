# Upgrade Guides

## 1.x -> 2.x

- Change `version: v1` to `version: v2` in your workload files
- Manifests are now written to a single file called `manifest.yaml` for each deployment.
  
  The directory structure of the output changed from:

    ```shell
    manifests
    └── edge-eu-w4
        └── my-app
            └── nginx-chart
                └── templates
    ```

  To:

    ```shell
    manifests
    └── edge
       └── my-app
          └── manifest.yaml
    ```
  
  The former behavior has been completely removed.

- All paths in the workloads files are now evaluated **relative to the file itself**, rather than relative to the location from where `heml-templexer` is called.
