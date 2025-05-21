import? 'cargo.just'

default:
  @just --list

fetch:
  curl https://raw.githubusercontent.com/Fuwn/justfiles/refs/heads/main/cargo.just > cargo.just
