import? 'cargo.just'

default:
  @just --list

fetch:
  curl https://raw.githubusercontent.com/Fuwn/justfiles/a6ca8a1b0475966ad10b68c44311ba3cb8b72a31/cargo.just > cargo.just
