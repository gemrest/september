VERSION 0.7

all:
  BUILD +docker
  BUILD +git

docker:
  ARG tag=latest

  FROM scratch

  COPY +build/september .

  EXPOSE 80

  CMD ["./september"]

  SAVE IMAGE --push fuwn/september:$tag

git:
  LOCALLY

  RUN git push

deps:
  ARG rustc="1.69.0"

  FROM clux/muslrust:$rustc

  RUN curl "https://static.rust-lang.org/rustup/archive/${RUSTUP_VER}/${RUST_ARCH}/rustup-init" -o rustup-init \
    && chmod +x rustup-init \
    && ./rustup-init -y --default-toolchain $rustc --profile minimal \
    && rm rustup-init \
    && ~/.cargo/bin/rustup target add x86_64-unknown-linux-musl \
    && echo "[build]\ntarget = \"x86_64-unknown-linux-musl\"" > ~/.cargo/config

  RUN apt-get update && apt-get install -y gnupg2

  RUN curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - \
    && apt-get install -y clang

build:
  FROM +deps

  WORKDIR /source

  RUN cargo new september

  WORKDIR /source/september

  COPY Cargo.* .

  RUN cargo build --release

  COPY .git .git
  COPY src src
  COPY build.rs build.rs
  COPY Cargo.* .

  RUN --mount=type=cache,target=/source/september/target \
      --mount=type=cache,target=/root/.cargo/registry \
      cargo build --release --bin september \
      && strip -s /source/september/target/x86_64-unknown-linux-musl/release/september \
      && mv /source/september/target/x86_64-unknown-linux-musl/release/september .

  SAVE ARTIFACT /source/september/september

