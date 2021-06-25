# Adbyss: Public Suffix

[![Documentation](https://docs.rs/adbyss_psl/badge.svg)](https://docs.rs/adbyss_psl/)
[![crates.io](https://img.shields.io/crates/v/adbyss_psl.svg)](https://crates.io/crates/adbyss_psl)
[![Build Status](https://github.com/Blobfolio/adbyss/workflows/Build/badge.svg)](https://github.com/Blobfolio/adbyss/actions)

This crate provides a very simple interface for checking hosts against the
[Public Suffix List](https://publicsuffix.org/list/).

This is a judgey library; hosts with unknown or missing suffixes are not parsed. No distinction is made between ICANN and private entries.

_Rules must be followed!_ Haha.

For hosts that do get parsed, their values will be normalized to lowercase ASCII.

Note: The master suffix data is baked into this crate at build time. This reduces the runtime overhead of parsing all that data out, but can also cause implementing apps to grow stale if they haven't been (re)packaged in a while.

## Examples

Initiate a new instance using `Domain::parse`. If that works, you then have accesses to the individual components:

```rust
use adbyss_psl::Domain;

let dom = Domain::parse("www.MyDomain.com").unwrap();
assert_eq!(dom.host(), "www.mydomain.com");
assert_eq!(dom.subdomain(), Some("www"));
assert_eq!(dom.root(), "mydomain");
assert_eq!(dom.suffix(), "com");
assert_eq!(dom.tld(), "mydomain.com");
```

A `Domain` object can be dereferenced to a string slice representing the sanitized host. You can also consume the object into an owned string with `Domain::take`.



## Installation

Add `adbyss_psl` to your `dependencies` in `Cargo.toml`, like:

```
[dependencies]
adbyss_psl = "0.3.*"
```



## License

Copyright © 2021 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

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
