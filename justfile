default:
  @just --list

fmt:
  cargo fmt

check:
  cargo check --all-features

checkf:
  @just fmt
  @just check

checkfc:
  @just checkf
  cargo clippy

run:
  @just checkfc
  cargo run
