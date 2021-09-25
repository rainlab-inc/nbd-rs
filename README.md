nbd-rs
======

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

