nbd-rs
======

[Network Block Device](https://en.wikipedia.org/wiki/Network_block_device) Server written in Rust. Currently in alpha stage. See `Disclaimer`.

## Why?

Main purpose of this project is to extend the capabilities of NBD Protocol. The same NBD Protocol you know, on steroids. Some of the extended features are:

  - Chainable backends (i.e. Cache, Retry)
  - Pluggable backends (Block: Raw, Sharded. Object: File, S3)
  - Memory-safety & Race-free NBD Server implementation, thanks to Rust

## Disclaimer

**DO NEVER USE THIS FOR PRODUCTION**

Do not use this for any data that you cannot afford to lose any moment. Expect data loss, corruption/bit rot, and every other possible storage disaster.

If you use alpha level software for your data, you might end up like we did previously => https://github.com/sheepdog/sheepdog/issues/425

*We're learning Rust, don't judge. Help instead.*

## Roadmap

### Done

* [X] Successfully serve dummy empty file (filled with zeroes), enough to satisfy `qemu-img info`
* [X] 0.0.1 Serve a raw image read-only from a file
  * [X] consider mmap
* [X] 0.0.2 Read/Write access
* [X] 0.0.3 Storage Backend Abstraction
* [X] 0.0.4 Sharded File Implementation. Shard image file into 4M chunks (my-image.{n}.chunk)
* [X] 0.0.5 Object Storage Abstraction
* [X] 0.0.6 S3 Object Storage
* [X] 0.0.7 Cache Backend Implementation

### v0.1.0 Alpha

* [ ] Known Issues (Fix before `v0.1.0`)
  * [ ] Fix panic at sudden disconnection
  * [ ] Fix panic at invalid export name
* [ ] Performance issues with Cache and S3 driver
  * [ ] Need multi-thread write workers (currently only a single extra thread)

### Backlog

* [ ] Retry Backend Implementation
* [ ] Trim Support
* [ ] Transmission Phase Refactor. Move to async/queue
  * currently all commands are executed serially, this severely effects performance
* [ ] Refactor `NBDSession`. Leverage Interior Mutability. (still learning Rust)
* [ ] Config/Argument syntax refactor. Consider having a config file as a last resort.
* [ ] Cache Storage refactor: split memory storage into a separate Object Storage driver
  * this will potentially allow using a file based cache layer (eg. on NVMe)

* [ ] Test Suite
  * [ ] Unit Tests
  * [ ] Integration Tests

Stretch goals

* [ ] Multi-volume support
* [ ] Multi-connection support
* [ ] Research Disconnect/Reconnection behavior
* [ ] Stateless Multi-server support
  * using Redis for state cache and coordination
* [ ] Encryption
* [ ] Sync and Async Mirrored backends
* [ ] Erasure coded backend
* [ ] HTTP backend
* [ ] Overlay (to overlay backends on top of each other, like using a snapshot)

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
    -e disk0 raw cache:file:/test/
    -e disk1 sharded cache:file:/test/
    -e disk2 raw s3:http://username:password@${S3_HOST}/bucket
    -e disk4 sharded cache:s3:http://username:password@${S3_HOST}/bucket
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

## Contributing

VERY WELCOME! *(Contributions to the contribution guide is also very welcome.)*

Please see [CONTRIBUTING.md](CONTRIBUTING.md)

## COPYING

[GPL-3.0](LICENSE)

Copyright 2021, Rainlab Inc. Tokyo and nbd-rs contributors (please see commit history)
