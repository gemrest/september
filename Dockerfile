FROM clux/muslrust:nightly-2022-03-08 AS environment

ENV CHANNEL=nightly-2022-03-08

RUN curl "https://static.rust-lang.org/rustup/archive/${RUSTUP_VER}/${RUST_ARCH}/rustup-init" -o rustup-init \
   && chmod +x rustup-init \
   && ./rustup-init -y --default-toolchain ${CHANNEL} --profile minimal \
   && rm rustup-init \
   && ~/.cargo/bin/rustup target add x86_64-unknown-linux-musl \
   && echo "[build]\ntarget = \"x86_64-unknown-linux-musl\"" > ~/.cargo/config

RUN cargo install sccache

RUN curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - \
    && apt-get update \
    && apt-get install -y clang

# RUN cargo install --git https://github.com/dimensionhq/fleet fleet-rs

FROM environment as builder

WORKDIR /usr/src

RUN cargo new september

WORKDIR /usr/src/september

COPY Cargo.* .

# RUN fleet build --release
RUN cargo build --release

COPY . .

RUN --mount=type=cache,target=/usr/src/september/target \
    --mount=type=cache,target=/root/.cargo/registry \
    cargo build --release --bin september \
    && strip -s /usr/src/september/target/x86_64-unknown-linux-musl/release/september \
    && mv /usr/src/september/target/x86_64-unknown-linux-musl/release/september .

FROM scratch

WORKDIR /september

COPY --from=builder /usr/src/september/september .

EXPOSE 80

ENTRYPOINT ["/september/september"]
