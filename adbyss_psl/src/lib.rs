/*!
# Adbyss: Public Suffix

This crate provides a very simple interface for checking hosts against the
[Public Suffix List](https://publicsuffix.org/list/).

This is a judgey library; hosts with unknown or missing suffixes are not
parsed. No distinction is made between ICANN and private entries. Rules must be
followed! Haha.

For hosts that do get parsed, their values will be normalized to lowercase
ASCII.

Note: The master suffix data is baked into this crate at build time. This reduces the
runtime overhead of parsing all that data out, but can also cause implementing
apps to grow stale if they haven't been packed lately.

## Example

Initiate a new instance using [`Domain::parse`]. If that works, you then have
accesses to the individual components:

```no_run
use adbyss_psl::Domain;

let dom = Domain::parse("www.MyDomain.com").unwrap();
assert_eq!(dom.host(), "www.mydomain.com");
assert_eq!(dom.subdomain(), Some("www"));
assert_eq!(dom.root(), "mydomain");
assert_eq!(dom.suffix(), "com");
assert_eq!(dom.tld(), "mydomain.com");
```

A [`Domain`] object can be dereferenced to a string slice of the sanitized
host. You can also consume the object into an owned string with [`Domain::take`].
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



mod list;
use self::list::{
	PSL_MAIN,
	PSL_WILD,
};

use std::hash::{
	Hash,
	Hasher,
};

use std::ops::{
	Deref,
	Range,
};



#[derive(Debug, Default, Clone)]
/// # Domain.
pub struct Domain {
	host: String,
	root: Range<usize>,
	suffix: Range<usize>,
}

impl Deref for Domain {
	type Target = str;
	#[inline]
	fn deref(&self) -> &Self::Target { &self.host }
}

impl Eq for Domain {}

impl Hash for Domain {
	fn hash<H: Hasher>(&self, state: &mut H) { self.host.hash(state); }
}

impl PartialEq for Domain {
	fn eq(&self, other: &Self) -> bool { self.host == other.host }
}

impl PartialEq<str> for Domain {
	fn eq(&self, other: &str) -> bool { self.host == other }
}

impl PartialEq<String> for Domain {
	fn eq(&self, other: &String) -> bool { self.host.eq(other) }
}

impl Domain {
	#[allow(clippy::option_if_let_else)] // Strings aren't `Copy`.
	/// # Parse Host.
	///
	/// Try to parse a given host. If the result has both a (valid) suffix and
	/// a root chunk (i.e. it has a TLD), a `Domain` object will be returned.
	///
	/// Hosts with unknown or missing suffixes are rejected. Otherwise all
	/// values are normalized to lowercase ASCII.
	pub fn parse<S>(src: S) -> Option<Self>
	where S: AsRef<str> {
		idna::domain_to_ascii_strict(src.as_ref().trim_matches(|c: char| c == '.' || c.is_ascii_whitespace()))
			.ok()
			.and_then(|host| parse_suffix(&host)
				.map(|s|
					// This has a subdomain, i.e. the root is in the middle.
					if let Some(d) = host.as_bytes()
						.iter()
						.take(s - 1)
						.rposition(|x| x == &b'.')
					{
						Self {
							root: d + 1..s - 1,
							suffix: s..host.len(),
							host,
						}
					}
					// The root starts at zero.
					else {
						Self {
							root: 0..s - 1,
							suffix: s..host.len(),
							host,
						}
					}
				)
			)
	}

	/// # Strip Leading WWW.
	///
	/// If this host has a leading `www.` subdomain, it will be removed. If it
	/// doesn't, no change. `True` is returned if a change is made.
	pub fn strip_www(&mut self)	-> bool {
		if self.root.start >= 4 && self.host.starts_with("www.") {
			unsafe { self.host.as_mut_vec().drain(0..4); }
			self.root.start -= 4;
			self.root.end -= 4;
			self.suffix.start -= 4;
			self.suffix.end -= 4;
			true
		}
		else { false }
	}

	#[allow(clippy::missing_const_for_fn)] // Doesn't work.
	#[must_use]
	/// # Into String.
	///
	/// Consume the struct, returning the sanitized host as an owned string.
	pub fn take(self) -> String { self.host }

	#[must_use]
	/// # Host.
	///
	/// Return the sanitized host as a string slice. This is equivalent to
	/// dereferencing the object.
	pub fn host(&self) -> &str { &self.host }

	#[must_use]
	/// # Root.
	///
	/// Return the root portion of the host, if any. This does not include any
	/// leading or trailing periods.
	pub fn root(&self) -> &str {
		&self.host[self.root.start..self.root.end]
	}

	#[must_use]
	/// # Subdomain(s).
	///
	/// Return the subdomain portion of the host, if any. This does not include
	/// any trailing periods.
	pub fn subdomain(&self) -> Option<&str> {
		if self.root.start > 0 { Some(&self.host[0..self.root.start - 1]) }
		else { None }
	}

	#[must_use]
	/// # Suffix.
	///
	/// Return the suffix of the host. This does not include any leading
	/// periods.
	pub fn suffix(&self) -> &str {
		&self.host[self.suffix.start..self.suffix.end]
	}

	#[must_use]
	/// # TLD.
	///
	/// Return the TLD portion of the host, i.e. everything but the
	/// subdomain(s).
	pub fn tld(&self) -> &str {
		&self.host[self.root.start..]
	}
}

/// # Find Suffix.
///
/// The hardest part of suffix validation is teasing the suffix out of the
/// hostname. Odd.
///
/// The suffix cannot be the whole of the thing, but should be the biggest
/// matching chunk of the host.
///
/// If a match is found, the starting index of the suffix (after its dot) is
/// returned.
fn parse_suffix(host: &str) -> Option<usize> {
	let bytes: &[u8] = host.as_bytes();
	let len: usize = host.len();
	if len < 3 || PSL_WILD.contains_key(host) || PSL_MAIN.contains(host) { return None; }

	let mut idx: usize = 0;
	let mut dot: usize = 0;
	while idx < len {
		if bytes[idx] != b'.' {
			idx += 1;
			continue;
		}

		// This is a wild extension.
		if let Some(exceptions) = PSL_WILD.get(&host[idx + 1..]) {
			// Our last chunk might start at zero instead of dot-plus-one.
			let after_dot: usize =
				if dot == 0 { 0 }
				else { dot + 1 };

			// This matches an exception, making the found suffix the true
			// suffix.
			if exceptions.contains(&&host[after_dot..idx]) {
				return Some(idx + 1);
			}
			// There has to be a before-before part.
			else if dot == 0 { return None; }
			// Otherwise the last chunk is part of the suffix.
			else {
				return Some(after_dot);
			}
		}
		// This is a normal suffix.
		else if PSL_MAIN.contains(&host[idx + 1..]) {
			return Some(idx + 1);
		}

		dot = idx;
		idx += 1;
	}

	None
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// # Test TLD Parsing.
	///
	/// These tests are adopted from the PSL [test data](https://raw.githubusercontent.com/publicsuffix/list/master/tests/test_psl.txt).
	fn t_tld() {
		// Mixed case.
		t_tld_assert("COM", None);
		t_tld_assert("example.COM", Some("example.com"));
		t_tld_assert("WwW.example.COM", Some("example.com"));
		// Leading dot.
		t_tld_assert(".com", None);
		t_tld_assert(".example", None);
		t_tld_assert(".example.com", Some("example.com"));
		t_tld_assert(".example.example", None);
		// Unlisted TLD.
		t_tld_assert("example", None);
		t_tld_assert("example.example", None);
		t_tld_assert("b.example.example", None);
		t_tld_assert("a.b.example.example", None);
		// TLD with only 1 rule.
		t_tld_assert("biz", None);
		t_tld_assert("domain.biz", Some("domain.biz"));
		t_tld_assert("b.domain.biz", Some("domain.biz"));
		t_tld_assert("a.b.domain.biz", Some("domain.biz"));
		// TLD with some 2-level rules.
		t_tld_assert("com", None);
		t_tld_assert("example.com", Some("example.com"));
		t_tld_assert("b.example.com", Some("example.com"));
		t_tld_assert("a.b.example.com", Some("example.com"));
		t_tld_assert("uk.com", None);
		t_tld_assert("example.uk.com", Some("example.uk.com"));
		t_tld_assert("b.example.uk.com", Some("example.uk.com"));
		t_tld_assert("a.b.example.uk.com", Some("example.uk.com"));
		t_tld_assert("test.ac", Some("test.ac"));
		// TLD with only 1 (wildcard) rule.
		t_tld_assert("mm", None);
		t_tld_assert("c.mm", None);
		t_tld_assert("b.c.mm", Some("b.c.mm"));
		t_tld_assert("a.b.c.mm", Some("b.c.mm"));
		// More complex TLD.
		t_tld_assert("jp", None);
		t_tld_assert("test.jp", Some("test.jp"));
		t_tld_assert("www.test.jp", Some("test.jp"));
		t_tld_assert("ac.jp", None);
		t_tld_assert("test.ac.jp", Some("test.ac.jp"));
		t_tld_assert("www.test.ac.jp", Some("test.ac.jp"));
		t_tld_assert("kyoto.jp", None);
		t_tld_assert("test.kyoto.jp", Some("test.kyoto.jp"));
		t_tld_assert("ide.kyoto.jp", None);
		t_tld_assert("b.ide.kyoto.jp", Some("b.ide.kyoto.jp"));
		t_tld_assert("a.b.ide.kyoto.jp", Some("b.ide.kyoto.jp"));
		t_tld_assert("c.kobe.jp", None);
		t_tld_assert("b.c.kobe.jp", Some("b.c.kobe.jp"));
		t_tld_assert("a.b.c.kobe.jp", Some("b.c.kobe.jp"));
		t_tld_assert("city.kobe.jp", Some("city.kobe.jp"));
		t_tld_assert("www.city.kobe.jp", Some("city.kobe.jp"));
		// TLD with a wildcard rule and exceptions.
		t_tld_assert("ck", None);
		t_tld_assert("test.ck", None);
		t_tld_assert("b.test.ck", Some("b.test.ck"));
		t_tld_assert("a.b.test.ck", Some("b.test.ck"));
		t_tld_assert("www.ck", Some("www.ck"));
		t_tld_assert("www.www.ck", Some("www.ck"));
		// US K12.
		t_tld_assert("us", None);
		t_tld_assert("test.us", Some("test.us"));
		t_tld_assert("www.test.us", Some("test.us"));
		t_tld_assert("ak.us", None);
		t_tld_assert("test.ak.us", Some("test.ak.us"));
		t_tld_assert("www.test.ak.us", Some("test.ak.us"));
		t_tld_assert("k12.ak.us", None);
		t_tld_assert("test.k12.ak.us", Some("test.k12.ak.us"));
		t_tld_assert("www.test.k12.ak.us", Some("test.k12.ak.us"));
		// IDN labels.
		t_tld_assert("食狮.com.cn", Some("xn--85x722f.com.cn"));
		t_tld_assert("食狮.公司.cn", Some("xn--85x722f.xn--55qx5d.cn"));
		t_tld_assert("www.食狮.公司.cn", Some("xn--85x722f.xn--55qx5d.cn"));
		t_tld_assert("shishi.公司.cn", Some("shishi.xn--55qx5d.cn"));
		t_tld_assert("公司.cn", None);
		t_tld_assert("食狮.中国", Some("xn--85x722f.xn--fiqs8s"));
		t_tld_assert("www.食狮.中国", Some("xn--85x722f.xn--fiqs8s"));
		t_tld_assert("shishi.中国", Some("shishi.xn--fiqs8s"));
		t_tld_assert("中国", None);
	}

	/// # Handle TLD Assertions.
	///
	/// The list is so big, it's easier to handle the testing in one place.
	fn t_tld_assert(a: &str, b: Option<&str>) {
		// The test should fail.
		if b.is_none() {
			let res = Domain::parse(a);
			assert!(
				res.is_none(),
				"Unexpectedly parsed: {:?}\n{:?}\n", a, res
			);
		}
		// We should have a TLD!
		else {
			if let Some(dom) = Domain::parse(a) {
				assert_eq!(
					dom.tld(),
					b.unwrap(),
					"Failed parsing: {:?}", dom
				);
			}
			else {
				panic!("Failed parsing: {:?}", a);
			}
		}
	}

	#[test]
	/// # Test Chunks.
	///
	/// This makes sure that the individual host components line up correctly.
	fn t_chunks() {
		let mut dom = Domain::parse("abc.www.食狮.中国").unwrap();
		assert_eq!(dom.subdomain(), Some("abc.www"));
		assert_eq!(dom.root(), "xn--85x722f");
		assert_eq!(dom.suffix(), "xn--fiqs8s");
		assert_eq!(dom.tld(), "xn--85x722f.xn--fiqs8s");
		assert_eq!(dom.deref(), "abc.www.xn--85x722f.xn--fiqs8s");
		assert_eq!(dom.host(), "abc.www.xn--85x722f.xn--fiqs8s");

		dom = Domain::parse("blobfolio.com").unwrap();
		assert_eq!(dom.subdomain(), None);
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.deref(), "blobfolio.com");
		assert_eq!(dom.host(), "blobfolio.com");

		dom = Domain::parse("www.blobfolio.com").unwrap();
		assert_eq!(dom.subdomain(), Some("www"));
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.deref(), "www.blobfolio.com");
		assert_eq!(dom.host(), "www.blobfolio.com");

		assert!(dom.strip_www());
		assert_eq!(dom.subdomain(), None);
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.deref(), "blobfolio.com");
		assert_eq!(dom.host(), "blobfolio.com");
	}
}
