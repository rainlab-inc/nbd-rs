FROM rust:1 as builder
WORKDIR /usr/src/nbd-rs
COPY . .
RUN cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/nbd-rs /usr/local/bin/nbd-rs
ENTRYPOINT [ "/usr/local/bin/nbd-rs" ]
