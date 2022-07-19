## Release Notes

### v0.1.0

* [x] New Features
  * [x] Distributed Block Storage
  * [x] Subcommands

* [x] Known Issues (Fix before `v0.1.0`)
  * [x] Fix panic at sudden disconnection
  * [x] Fix panic at invalid export name
* [x] Performance issues with Cache and S3 driver
  * [ ] Need multi-thread write workers (currently only a single extra thread)

### Backlog

* [ ] Transmission Phase Refactor. Move to async/queue
  * currently all commands are executed serially, this severely effects performance
* [ ] Refactor `NBDSession`. Leverage Interior Mutability. (still learning Rust)
* [ ] Cache Storage refactor: split memory storage into a separate Object Storage driver
  * this will potentially allow using a file based cache layer (eg. on NVMe)

* [ ] Test Suite
  * [ ] Unit Tests
  * [ ] Integration Tests

Stretch goals

* [x] ~~Multi-volume support~~
  * Consider dynamic volume support? (created on demand)
* [ ] Multi-connection support
* [ ] Research Disconnect/Reconnection behavior
* [ ] Stateless Multi-server support
  * using Redis for state cache and coordination
* [ ] Encryption
* [ ] Sync and Async Mirrored backends
* [ ] Erasure coded backend
* [ ] HTTP backend (simpler approach to object storage, compared to S3)
* [ ] Overlay (to overlay backends on top of each other, like using a snapshot)
