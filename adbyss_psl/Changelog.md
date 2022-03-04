# Changelog



## [0.7.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.7.0) - TBD

### Removed

* `Domain::parse`



## [0.6.2](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.2) - 2022-02-08

### Changes

* Improved documentation;



## [0.6.1](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.1) - 2022-01-19

### Changes

* 2x-3x performance improvements for PUNY and Unicode domain parsing;
* Build script cleanup;
* Import IDNA/Unicode unit tests;



## [0.6.0](https://github.com/Blobfolio/adbyss/releases/tag/v0.6.0) - 2022-01-13

### New

* `Domain::strip_www`;
* `Domain` impls:
  * `AsRef<[u8]>`
  * `PartialEq<&str>`
  * `PartialEq<String>`

### Changes

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
