nbd-rs
======

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

## Container

```sh
docker build -t dkr.local/nbd-rs:dev .
docker run -it --rm -p 10809:10809 dkr.local/nbd-rs:dev
```

## Test

```sh
qemu-img info nbd:localhost:10809;exportname=default
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
qemu-system-x86_64   -enable-kvm   -machine q35,accel=kvm   -m 2048  -drive file=nbd:127.0.0.1:10809:exportname=zeroimage,format=raw   -display gtk   -serial mon:stdio
```

## COPYING

[GPL-3.0](LICENSE)

Copyright 2021, Rainlab Inc. Tokyo
