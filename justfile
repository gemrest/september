import? 'cargo.just'

set allow-duplicate-recipes := true

name := "september"
ghcr_repo := "ghcr.io/gemrest/" + name
gitlab_repo := "registry.gitlab.com/gemrest/" + name
docker_hub_repo := "docker.io/fuwn/" + name

default:
  @just --list

fetch:
  curl https://raw.githubusercontent.com/Fuwn/justfiles/a6ca8a1b0475966ad10b68c44311ba3cb8b72a31/cargo.just > cargo.just

fmt:
  cargo +nightly fmt

# Build both architectures, then publish `latest` and the latest git tag.
publish-images:
  #!/usr/bin/env bash

  set -euo pipefail

  git_tag="$(git describe --tags --abbrev=0)"
  docker_tag="${git_tag#v}"

  docker buildx build \
    --platform linux/amd64,linux/arm64 \
    --file Dockerfile \
    --tag "{{ghcr_repo}}:latest" \
    --tag "{{ghcr_repo}}:${docker_tag}" \
    --tag "{{gitlab_repo}}:latest" \
    --tag "{{gitlab_repo}}:${docker_tag}" \
    --tag "{{docker_hub_repo}}:latest" \
    --tag "{{docker_hub_repo}}:${docker_tag}" \
    --push \
    .
