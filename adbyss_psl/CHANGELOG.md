# Changelog



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
