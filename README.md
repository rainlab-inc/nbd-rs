nbd-rs
======

## Disclaimer

**DO NEVER USE THIS FOR PRODUCTION**

Do not use this for any data that you cannot afford to lose any moment. Expect data loss, corruption/bit rot, and every other possible storage disaster.

If you use alpha level software for your data, you might end up like we did previously => https://github.com/sheepdog/sheepdog/issues/425

## Roadmap

* [X] Successfully serve dummy empty file (filled with zeroes), enough to satisfy `qemu-img info`
* [X] 0.0.1 Serve a raw image read-only from a file
  * [X] consider mmap
* [X] 0.0.2 Read/Write access
  * [ ] ~~Trim support (remove entire chunk object if trimmed)~~
* [X] 0.0.3 Storage Backend Abstraction
* [X] 0.0.4 Sharded File Implementation. Shard image file into 4M chunks (my-image.{n}.chunk)
* [X] 0.0.5 Object Storage Abstraction
* [X] 0.0.6 S3 Object Storage
* [X] 0.0.7 Cache Backend Implementation
* [ ] Retry Backend Implementation
* [ ] Trim Support
* [ ] Transmission Phase Refactor. Move to async/queue
* [ ] Refactor `NBDSession`. Leverage Interior Mutability.
* [ ] Config/Argument syntax refactor.
* [ ] Memory-based Object Storage
* [ ] Test Suite
  * [ ] Unit Tests
  * [ ] Integration Tests
* [ ] Known Issues
  * [ ] Fix panic at sudden disconnection
  * [ ] Fix panic at invalid export name

Stretch goals

* [ ] Speed & BW optimizations, lazy disk-write, page-cache, etc.
* [ ] Multi-volume support
* [ ] Multi-connection support
* [ ] Research Disconnect/Reconnection behavior
* [ ] S3 backing support for shards
* [ ] Stateless Multi-server support (S3 & Redis backend)
  * Redis: state cache, etc. S3: storage
* [ ] Encryption

## Build

```sh
cargo build
```

The executable binary is located at `./target/debug/nbd-rs`.

## Run

Arguments:

```
[-e | --export EXPORT_NAME; DRIVER (raw, sharded); (cache)? CONN_STR]...
  Examples:
    -e my_export raw cache:file:/test/
    -e my_export sharded cache:file:/test/
    -e my_export raw cache:s3:http://username:password@${S3_HOST}/path
    -e my_export raw s3:http://username:password@${S3_HOST}/path
    -e my_export sharded s3:http://username:password@${S3_HOST}/path
```

Examples of export argument:

```sh
# Single Export (-e | --export), Raw, File, Log Level: DEBUG
RUST_LOG=debug ./target/debug/nbd-rs --export my_raw_export raw file:/export/path/

# Single Export, Sharded, S3, Log Level: TRACE
RUST_LOG=trace ./target/debug/nbd-rs -e my_raw_export sharded s3:/export/path/

# Single Export with Cache, Sharded, S3
./target/debug/nbd-rs -e my_sharded_export sharded cache:s3:http://username:password@${S3_HOST}/path

# Multiple Exports
./target/debug/nbd-rs -e my_raw_export raw file:/export/path/ -e my_sharded_export sharded s3:http://username:password@${S3_HOST}/path
```

**NBD-rs will panic if no export has been specified.**

## Container

```sh
docker build -t dkr.local/nbd-rs:dev .
docker run -it --rm -p 10809:10809 dkr.local/nbd-rs:dev --export ${EXPORT_NAME} raw file:/test/
```

> See `Run` section for more information on arguments.

## Test

```sh
qemu-img info nbd:localhost:10809;exportname=${EXPORT_NAME}
```

Write local image to NBD:

```sh
# Connect to drive by reading
nbd-client -N ${IMAGE_NAME} localhost /dev/nbd0

# Write
dd if=${LOCAL_IMAGE} of=/dev/nbd0 bs=1M status=progress oflag=sync,direct

# Disconnect
nbd-client -d /dev/nbd0
```

Boot alpine with qemu:

```sh
qemu-system-x86_64   -enable-kvm   -machine q35,accel=kvm   -m 2048  -drive file=nbd:127.0.0.1:10809:exportname=${EXPORT_NAME},format=raw   -display gtk   -serial mon:stdio
```

## COPYING

[GPL-3.0](LICENSE)

Copyright 2021, Rainlab Inc. Tokyo
