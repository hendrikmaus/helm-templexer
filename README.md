# Helm Templexer

Render Helm charts for multiple environments with explicit config while keeping the overhead at ease.

## Introduction

This could be for you if you commonly run into a scenario that requires to render Helm chart(s) for multiple environments, while not using Helm as your actual deployment manager, for example you might prefer to `kubectl apply` the manifests yourself or have a gitops operator do that for you.

Helm Templexer deals with a single explicit configuration file for each workload you want to run across your Kubernetes cluster(s).

