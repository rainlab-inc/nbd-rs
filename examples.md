# Tested Subcommands

## Raw

``` shell
nbd-rs init --size 100Mi raw "file:$(pwd)/raw.bin"
nbd-rs init --size 200Mi --force raw "file:$(pwd)/raw.bin"
nbd-rs serve --export mydisk raw "file:$(pwd)/raw.bin"
nbd-rs destroy raw "file:$(pwd)/raw.bin"
```

In a different folder(there was a path bug):

``` shell
nbd-rs init --size 100Mi raw "file:$(pwd)/export/raw.bin"
nbd-rs init --size 200Mi --force raw "file:$(pwd)/export/raw.bin"
nbd-rs serve --export mydisk raw "file:$(pwd)/export/raw.bin"
nbd-rs destroy raw "file:$(pwd)/export/raw.bin"
```

## Distributed

### File

``` shell
nbd-rs init --size 4Gi distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs init --size 5Gi --force distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs serve --export mydisk distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs destroy distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
```

### S3

``` shell
nbd-rs init --size 2Gi distributed "replicas=3;backends=\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node0,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node1,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node2,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node3;"

nbd-rs init --size 1Gi --force distributed "replicas=3;backends=\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node0,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node1,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node2,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node3;"

nbd-rs serve --export mydisk distributed "replicas=3;backends=\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node0,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node1,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node2,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node3;"

nbd-rs destroy distributed "replicas=3;backends=\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node0,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node1,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node2,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node3;"
```

## Serve ISO image as a RAW disk, and boot it using Qemu

You might need to add your user to `kvm` group

```
sudo adduser $USER kvm
# or depending on the distro
# sudo usermod -a -G kvm $USER
```

### Download Alpine Linux 3.15 iso image

```shell
wget "https://dl-cdn.alpinelinux.org/alpine/v3.15/releases/x86_64/alpine-virt-3.15.0-x86_64.iso"
```

## Run NBD-RS using Docker

```shell
docker build -t dkr.local/nbd-rs:dev .
docker run -it --rm -p 10809:10809 \
  -v ${ISO_IMAGE_LOCATION}:/opt/nbd \
  -e RUST_BACKTRACE=full \
  dkr.local/nbd-rs:dev \
  --export alpine-image raw file:/opt/nbd/alpine-virt-3.15.0-x86_64.iso
```


## Boot VM using Qemu

```shell
qemu-system-x86_64 \
  -enable-kvm -machine q35,accel=kvm -m 2048 \
  -drive file=nbd:127.0.0.1:10809:exportname=alpine-image,format=raw \
  -display none -serial mon:stdio
```

Use `CTRL+a`, and then `x` to terminate qemu process.
