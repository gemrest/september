VERSION 0.7

image:
  ARG tag=latest

  FROM scratch

  COPY +build/september .

  EXPOSE 80

  CMD ["./september"]

  SAVE IMAGE --push fuwn/september:$tag

build:
  FROM messense/rust-musl-cross:x86_64-musl

  WORKDIR /source

  RUN cargo new september

  WORKDIR /source/september

  COPY Cargo.* .

  RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

  COPY .git .git
  COPY src src
  COPY build.rs build.rs
  COPY Cargo.* .
  COPY default.css .

  RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
  RUN strip -s /source/september/target/x86_64-unknown-linux-musl/release/september
  RUN mv /source/september/target/x86_64-unknown-linux-musl/release/september .

  SAVE ARTIFACT /source/september/september
