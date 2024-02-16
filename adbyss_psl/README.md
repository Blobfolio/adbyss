# Adbyss: Public Suffix

[![docs.rs](https://img.shields.io/docsrs/adbyss_psl.svg?style=flat-square&label=docs.rs)](https://docs.rs/adbyss_psl/)
[![changelog](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=changelog&color=9b59b6)](https://github.com/Blobfolio/adbyss/blob/master/adbyss_psl/CHANGELOG.md)<br>
[![crates.io](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=crates.io)](https://crates.io/crates/adbyss_psl)
[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/adbyss/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/adbyss/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/adbyss/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/adbyss)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/adbyss/issues)

This library contains a single public-facing struct — `adbyss_psl::Domain` — used for validating and normalizing Internet hostnames, like "www.domain.com".

It will:
* Validate, normalize, and Puny-encode internationalized/Unicode labels ([RFC 3492](https://datatracker.ietf.org/doc/html/rfc3492#ref-IDNA));
* Validate and normalize the [public suffix](https://publicsuffix.org/list/);
* Ensure conformance with [RFC 1123](https://datatracker.ietf.org/doc/html/rfc1123);
* And locate the boundaries of the subdomain (if any), root (required), and suffix (required); 

Suffix and IDNA reference data is compiled at build-time, allowing for very fast runtime parsing, but at the cost of _temporality_. Projects using this library will need to periodically issue new releases or risk growing stale.



## Examples

New instances of `Domain` can be initialized using either `Domain::new` or `TryFrom<&str>`.

```rust
use adbyss_psl::Domain;

// These are equivalent and fine:
assert!(Domain::new("www.MyDomain.com").is_some());
assert!(Domain::try_from("www.MyDomain.com").is_ok());

// The following is valid DNS, but invalid as an Internet hostname:
assert!(Domain::new("_acme-challenge.mydomain.com").is_none());
```

Valid Internet hostnames must be no longer than 253 characters, and contain both root and (valid) suffix components.

Their labels — the bits between the dots — must additionally:
* Be no longer than 63 characters;
* (Ultimately) contain only ASCII letters, digits, and `-`;
* Start and end with an alphanumeric character;

Unicode/internationalized labels are allowed, but must be Puny-encodable and not contain any conflicting bidirectionality constraints. `Domain` will encode such labels using [Punycode](https://en.wikipedia.org/wiki/Punycode) when it finds them, ensuring the resulting hostname will always be ASCII-only.

Post-parsing, `Domain` gives you access to each individual component, or the whole thing:

```rust
use adbyss_psl::Domain;

let dom = Domain::new("www.MyDomain.com").unwrap();

// Pull out the pieces if you're into that sort of thing.
assert_eq!(dom.host(), "www.mydomain.com");
assert_eq!(dom.subdomain(), Some("www"));
assert_eq!(dom.root(), "mydomain");
assert_eq!(dom.suffix(), "com");
assert_eq!(dom.tld(), "mydomain.com");

// If you just want the sanitized host back as an owned value, use
// `Domain::take`:
let owned = dom.take(); // "www.mydomain.com"
```



## Optional Crate Features

* `serde`: Enables serialization/deserialization support.



## Installation

Add `adbyss_psl` to your `dependencies` in `Cargo.toml`, like:

```
[dependencies]
adbyss_psl = "0.10.*"
```



## License

Copyright © 2024 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004
    
    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>
    
    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.
    
    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
    
    0. You just DO WHAT THE FUCK YOU WANT TO.
