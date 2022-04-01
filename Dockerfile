FROM rust:latest as builder

RUN update-ca-certificates

WORKDIR /september

COPY ./ ./

RUN cargo build --release

RUN strip -s /september/target/release/september

FROM debian:buster-slim

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /september

COPY --from=builder /september/target/release/september ./

EXPOSE 80

CMD ["./september"]
