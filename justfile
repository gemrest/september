import? 'cargo.just'

set allow-duplicate-recipes := true

name := "september"
ghcr_repo := "ghcr.io/gemrest/" + name
gitlab_repo := "registry.gitlab.com/gemrest/" + name
docker_hub_repo := "docker.io/gemrest/" + name

default:
  @just --list

fetch:
  curl https://raw.githubusercontent.com/Fuwn/justfiles/a6ca8a1b0475966ad10b68c44311ba3cb8b72a31/cargo.just > cargo.just

fmt:
  cargo +nightly fmt

# Build once, then push both `latest` and the latest git tag to all registries.
publish-images:
  #!/usr/bin/env bash

  set -euo pipefail

  git_tag="$(git describe --tags --abbrev=0)"
  docker_tag="${git_tag#v}"

  docker build --platform linux/amd64 -f Dockerfile -t {{name}}:build .

  for registry in {{ghcr_repo}} {{gitlab_repo}} {{docker_hub_repo}}; do
    docker tag {{name}}:build "$registry:latest"
    docker tag {{name}}:build "$registry:$docker_tag"
  done

  docker push "{{ghcr_repo}}:latest"
  docker push "{{ghcr_repo}}:$docker_tag"
  docker push "{{gitlab_repo}}:latest"
  docker push "{{gitlab_repo}}:$docker_tag"
  docker push "{{docker_hub_repo}}:latest"
  docker push "{{docker_hub_repo}}:$docker_tag"
