FROM rust:1 as builder
WORKDIR /usr/src/nbd-proxy-rs
COPY . .
RUN cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/nbd-proxy-rs /usr/local/bin/nbd-proxy-rs
ENTRYPOINT [ "/usr/local/bin/nbd-proxy-rs" ]
