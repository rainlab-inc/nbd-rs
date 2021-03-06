nbd-rs
======

[Network Block Device](https://en.wikipedia.org/wiki/Network_block_device) Server written in Rust. Currently in alpha stage. See `Disclaimer`.

## Why?

Main purpose of this project is to explore possibilities around network block storage, using the capabilities of NBD Protocol, but beyond the capabilities of nbd-server. Some of the extended features are:

  - Pluggable backends (Block: Raw, Sharded, Distributed Object: File, S3)
  - Chainable backends (i.e. Cache, Retry)
  - Memory-safety & Race-free NBD Server implementation, thanks to Rust

## Disclaimer

**DO NEVER USE THIS FOR PRODUCTION**

Do not use this for any data that you cannot afford to lose any moment. Expect data loss, corruption/bit rot, and every other possible storage disaster.

If you use alpha level software for your data, you might end up like we did previously => https://github.com/sheepdog/sheepdog/issues/425

*We're learning Rust, don't judge. Help instead.*

## NOT FEATURES

* You will lose your data if you rely on this alpha software
  * Test it, OK
  * Use it, you will lose your data

## Features
  See [changelog](CHANGELOG.md).
* Ability to serve raw images
* Ability to serve chunked volumes (a raw image, split into 4MB chunks)
  * from various (pluggable) backends, currently `file` and `s3` backends are implemented
* Ability to use chainable backends, like `cache`
* Ability to distributed chunks to multiple backend storages

## General Architecture

TODO: Make a graph / drawing

* NBD Server
* -> serves an export(volume)
* -> uses a BlockStorage internally
  * could be a single RAW image (RawStorage)
  * could be a distributed volume (DistributedStorage)
* -> uses an ObjectStorage backend (could be chained)
  * could be a single file (mmap'ed) (FileObjectStorage) 
    `file:$(pwd)/raw.bin`
  * could be multiple files for DistributedStorage (FileObjectStorage)
    `file:$(pwd)/disk1/chunks/`
  * could be S3 for multiple files (S3ObjectStorage)
    `s3://minio:minio@localhost:9000/diskbucket/disk1/chunks/`
  * could use CacheStorage for memory cache (chained to something else above)
    `cache+s3://minio:minio@localhost:9000/diskbucket/disk1/chunks/`

## Build

```sh
cargo build
```

The executable binary is located at `./target/debug/nbd-rs`.

## Run

### Subcommands

```sh
nbd-rs init --size <SIZE> <DRIVER> <DRIVER_CFG>
nbd-rs serve --export <EXPORT> <DRIVER> <DRIVER_CFG>
nbd-rs destroy <DRIVER> <DRIVER_CFG>
```

### Simple Example

```sh
nbd-rs init --size 100Mi raw "file:$(pwd)/raw.bin"
nbd-rs serve --export mydisk raw "file:$(pwd)/raw.bin"
nbd-rs destroy raw "file:$(pwd)/raw.bin"
```

### Multiple Exports

```sh
nbd-rs init --size 100Mi raw "file:$(pwd)/raw.bin"
nbd-rs init --size 200Mi raw "file:$(pwd)/raw2.bin"
nbd-rs serve --export disk0 raw "file:$(pwd)/raw.bin" --export disk1 raw "file:$(pwd)/raw2.bin"
nbd-rs destroy raw "file:$(pwd)/raw.bin"
nbd-rs destroy raw "file:$(pwd)/raw2.bin"
```

### Distributed Example

```sh
nbd-rs init --size 2Gi distributed "replicas=3;backends=\
cache:s3:http://usename:password@${S3_HOST}/node0,\
cache:s3:http://usename:password@${S3_HOST}/node1;"
```

```sh
nbd-rs serve --export mydisk distributed "replicas=3;backends=\
cache:s3:http://username:password@${S3_HOST}/node0,\
cache:s3:http://username:password@${S3_HOST}/node1;"
```

```sh
nbd-rs destroy distributed "replicas=3;backends=\
cache:s3:http://username:password@${S3_HOST}/node0,\
cache:s3:http://username:password@${S3_HOST}/node1;"
```

For more advanced examples please look [examples.md](examples.md).

## Contributing

VERY WELCOME! *(Contributions to the contribution guide is also very welcome.)*

Please see [CONTRIBUTING.md](CONTRIBUTING.md)

## COPYING

[GPL-3.0](LICENSE)

Copyright 2021, Rainlab Inc. Tokyo and nbd-rs contributors (please see commit history)

