FROM rust:1 as builder
RUN apt-get update && apt-get install libssl1.1 libssl-dev
WORKDIR /usr/src/nbd-rs
COPY . .
RUN cargo install --path .

FROM debian:buster
RUN apt-get update && apt-get install libssl1.1
COPY --from=builder /usr/local/cargo/bin/nbd-rs /usr/local/bin/nbd-rs
ENTRYPOINT [ "/usr/local/bin/nbd-rs" ]
