FROM rust:1-alpine3.14 as builder
WORKDIR /usr/src/nbd-proxy-rs
COPY . .
RUN cargo install --path .

FROM alpine:3.14
RUN apk add tini
COPY --from=builder /usr/local/cargo/bin/nbd-proxy-rs /usr/local/bin/nbd-proxy-rs
ENTRYPOINT [ "/sbin/tini", "--", "/usr/local/bin/nbd-proxy-rs" ]
