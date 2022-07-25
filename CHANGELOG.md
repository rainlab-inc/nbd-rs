# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Implement `distributed` block storage.
- Implement subcommands:
  * init, serve, destroy
### Changed
- Fix panic at sudden disconnection.
- Fix panic at invalid export name.

## [0.0.7] - 2021-12-02
### Added
- Implement `cache` backend.
## [0.0.6] - 2021-11-20
### Added
- Implement `S3` object storage.
## [0.0.5] - 2021-11-20
### Added
- Implement Object Storage Abstraction.
## [0.0.4] - 2021-10-28
### Added
- Implement `sharded` block storage.
## [0.0.3] - 2021-10-26
### Added
- Make generic storage backends.
- Implement MmapBackend driver.
## [0.0.2] - 2021-10-22
### Added
- Implement `write`, `flush` and `block status` commands.
## [0.0.1] - 2021-10-20
### Added
- Read-only implementation of NBD Server. Capable of establishing single connection at a time. Works with `mmap`.

[Unreleased]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.7...master
[0.1.0]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.7...v0.1.0 
[0.0.7]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.6...v0.0.7 
[0.0.6]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.5...v0.0.6 
[0.0.5]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.4...v0.0.5 
[0.0.4]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.3...v0.0.4 
[0.0.3]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.2...v0.0.3
[0.0.2]: https://git.rlab.io/playground/nbd-rs/-/compare/v0.0.1...v0.0.2
[0.0.1]: https://git.rlab.io/playground/nbd-rs/-/releases/v0.0.1
