# Changelog


## [0.12.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.12.0) - 2024-08-08

### New

* `Domain::try_from::<Cow<str>>`

### Breaking

* Bump MSRV to `1.80`

### Changed

* Specialize `Domain::try_from::<String>`
* Add dependency `trimothy` (lib)
* Bump `utc2k` to `0.9`
* Update suffix database

### Fixed

* Remove old assets from `build.rs` `rerun-if-changed` triggers (lib)



## [0.11.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.11.1) - 2024-07-25

### Changed

* Update suffix database



## [0.11.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.11.0) - 2024-06-13

### Changed

* Use `idna` crate for puny/unicode handling
* Update suffix database



## [0.10.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.10.2) - 2024-05-02

### Changed

* Update suffix database



## [0.10.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.10.1) - 2024-03-21

### Changed

* Update suffix database



## [0.10.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.10.0) - 2024-02-15

### Changed

* Bump MSRV to `1.72`
* Update suffix database



## [0.9.5](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.5) - 2024-02-08

### Changed

* Update suffix database



## [0.9.4](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.4) - 2024-01-26

### Changed

* Update suffix database



## [0.9.3](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.3) - 2023-12-28

### Changed

* Bump `idna` to `0.5`



## [0.9.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.2) - 2023-11-16

### Changed

* Update suffix database



## [0.9.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.1) - 2023-10-09

### Changed

* Loosen build-dependency constraints for better downstream interoperability
* Bump MSRV to `1.65` to match the latest `regex` release (`1.10`), however Rust `1.63` can still be used if `regex` is capped between `1.7..=1.9`
* Update suffix database



## [0.9.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.9.0) - 2023-10-05

### Changed

* Minor code lints and cleanup
* Update suffix database



## [0.8.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.8.2) - 2023-08-24

### Changed

* Update suffix database



## [0.8.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.8.1) - 2023-07-05

### Changed

* Update dependencies



## [0.8.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.8.0) - 2023-06-01

### Changed

* Remove all `unsafe` code
* Improve unit test coverage
* Minor code changes and lints
* Drop `dactyl` build dependency



## [0.7.22](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.22) - 2023-04-27

### Changed

* Update dependencies



## [0.7.21](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.21) - 2023-04-20

### Fixed

* Unit test stack overflow in Rust `1.69`



## [0.7.20](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.20) - 2023-03-09

### Changed

* Deserialize from string slice instead of Cow
* impl `FromStr` for `Domain`



## [0.7.19](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.19) - 2023-02-04

### Changed

* Improve docs.rs environment detection
* Declare "serde" feature explicitly



## [0.7.18](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.18) - 2023-01-26

### Changed

* Bump brunch `0.4`
* Minor code lints



## [0.7.17](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.17) - 2022-12-26

### Changed

* Drop `ureq` build dependency; the remote data are now bundled with each release.



## [0.7.16](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.16) - 2022-12-15

(Bin-only release.)



## [0.7.15](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.15) - 2022-11-06

### Changed

* Bump Unicode 15
* Bump regex `1.7.0`



## [0.7.14](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.14) - 2022-11-03

### Changed

* Relax `ahash` version requirements
* Improved docs
* Minor code lints



## [0.7.13](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.13) - 2022-09-22

### Changed

* Update dependencies
* Improve docs



## [0.7.12](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.12) - 2022-09-11

### Changed

* Bump MSRV `1.63`
* Cleanup dependencies



## [0.7.11](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.11) - 2022-08-22

### Changed

* Update dependencies



## [0.7.10](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.10) - 2022-08-12

### Changed

* Go back to using `ahash` for internal keying



## [0.7.9](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.9) - 2022-08-11

### Changed

* Replace `ahash` with `wyhash` (the former no longer supports static keys)
* Remove `serde_yaml` dev dependency



## [0.7.8](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.8) - 2022-07-14

### Changed

* Update dependencies
* Loosen build dependency requirements



## [0.7.7](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.7) - 2022-06-30

### Changed

* Update dependencies



## [0.7.6](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.6) - 2022-06-18

### Changed

* Update dependencies



## [0.7.5](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.5) - 2022-05-30

### Changed

* Update dependencies



## [0.7.4](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.4) - 2022-05-19

### Changed

* Update and lock third-party dependency versions



## [0.7.3](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.3) - 2022-04-29

This release merely contains some documentation and linting changes.



## [0.7.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.2) - 2022-04-07

There are no changes from version 0.7.1; this was a binary-only update.



## [0.7.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.1) - 2022-03-29

### Changed

* Improve performance of `build.rs`
* Remove an `unsafe` block



## [0.7.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.0) - 2022-03-07

### Removed

* `Domain::parse`



## [0.6.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.2) - 2022-02-08

### Changed

* Improved documentation;



## [0.6.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.1) - 2022-01-19

### Changed

* 2x-3x performance improvements for PUNY and Unicode domain parsing;
* Build script cleanup;
* Import IDNA/Unicode unit tests;



## [0.6.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.0) - 2022-01-13

### Added

* `Domain::strip_www`;
* `Domain` impls:
  * `AsRef<[u8]>`
  * `PartialEq<&str>`
  * `PartialEq<String>`

### Changed

* 10x performance improvements for non-PUNY, non-Unicode domain parsing;
* IDNA/Unicode normalization is not handled internally;
* Bump IDNA/Unicode standard to `14.0.0`;
* Trailing and leading hyphens in domain parts are no longer allowed;
* Added `unicode-bidi` dependency;
* Added `unicode-normalization` dependency;
* Removed `idna` dependency;
* Removed `once_cell` dependency;

### Deprecated

* `Domain::parse` is being replaced by `Domain::new`;
