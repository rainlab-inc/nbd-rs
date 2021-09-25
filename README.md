nbd-rs
======

## Roadmap

* [ ] 0.1 Successfully serve dummy empty file (filled with zeroes), enough to satisfy `qemu-img info`
* [ ] 0.2 Serve a raw image read-only from a file
  * [ ] consider mmap
* [ ] 0.3 Read/Write access
* [ ] 0.4 Shard image file into 4M chunks (my-image.{n}.chunk)
  * [ ] Trim support (remove entire chunk object if trimmed)

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

## COPYING

[GPL-3.0](LICENSE)

Copyright 2021, Rainlab Inc. Tokyo

