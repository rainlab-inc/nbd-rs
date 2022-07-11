# Tested Subcommands
## Raw
``` shell
nbd-rs init --size 100M raw "file:$(pwd)/raw.bin"
nbd-rs init --size 200M --force raw "file:$(pwd)/raw.bin"
nbd-rs serve --export mydisk raw "file:$(pwd)/raw.bin"
nbd-rs destroy raw "file:$(pwd)/raw.bin"
```

In a different folder(there was a path bug):
``` shell
nbd-rs init --size 100M raw "file:$(pwd)/export/raw.bin"
nbd-rs init --size 200M --force raw "file:$(pwd)/export/raw.bin"
nbd-rs serve --export mydisk raw "file:$(pwd)/export/raw.bin"
nbd-rs destroy raw "file:$(pwd)/export/raw.bin"
```

## Distributed

### File
``` shell
nbd-rs init --size 4G distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs init --size 5G --force distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs serve --export mydisk distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
nbd-rs destroy distributed "replicas=1;backends=file:$(pwd)/export,file:$(pwd)/export2;"
```

### S3
``` shell
nbd-rs init --size 2G distributed "replicas=3;backends=\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node0,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node1,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node2,\
cache:s3:http://miniotest:miniotest@127.0.0.1:9000/node3;"

nbd-rs init --size 1G --force distributed "replicas=3;backends=\
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