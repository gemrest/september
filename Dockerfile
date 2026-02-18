# syntax=docker/dockerfile:1.7

ARG RUSTC_VERSION=stable

FROM clux/muslrust:${RUSTC_VERSION} AS build

WORKDIR /source

RUN cargo new september

WORKDIR /source/september

COPY Cargo.toml Cargo.lock ./

RUN --mount=type=cache,target=/root/.cargo/registry \
  --mount=type=cache,target=/root/.cargo/git \
  cargo build --release

COPY .git ./.git
COPY src ./src
COPY build.rs ./build.rs
COPY default.css .

RUN --mount=type=cache,target=/root/.cargo/registry \
  --mount=type=cache,target=/root/.cargo/git \
  cargo build --release --bin september
RUN set -eux; \
  bin_path="$(find /source/september/target -type f -path '*/release/september' | head -n 1)"; \
  test -n "${bin_path}"; \
  strip -s "${bin_path}"; \
  cp "${bin_path}" /source/september/september
RUN strip -s /source/september/september

FROM scratch AS runtime

WORKDIR /september

COPY --from=build /source/september/september ./september

EXPOSE 80

ENTRYPOINT ["./september"]
