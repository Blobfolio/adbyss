/*!
# Adbyss: Public Suffix

[![docs.rs](https://img.shields.io/docsrs/adbyss_psl.svg?style=flat-square&label=docs.rs)](https://docs.rs/adbyss_psl/)
[![changelog](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=changelog&color=9b59b6)](https://github.com/Blobfolio/adbyss/blob/master/adbyss_psl/CHANGELOG.md)<br>
[![crates.io](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=crates.io)](https://crates.io/crates/adbyss_psl)
[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/adbyss/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/adbyss/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/adbyss/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/adbyss)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/adbyss/issues)

This library contains a single public-facing struct — [`Domain`] — used for validating and normalizing Internet hostnames, like "www.domain.com".

It will:
* Validate, normalize, and Puny-encode internationalized/Unicode labels ([RFC 3492](https://datatracker.ietf.org/doc/html/rfc3492#ref-IDNA));
* Validate and normalize the [public suffix](https://publicsuffix.org/list/);
* Ensure conformance with [RFC 1123](https://datatracker.ietf.org/doc/html/rfc1123);
* And locate the boundaries of the subdomain (if any), root (required), and suffix (required);

Suffix and IDNA reference data is compiled at build-time, allowing for very fast runtime parsing, but at the cost of _temporality_. Projects using this library will need to periodically issue new releases or risk growing stale.



## Examples

New instances of [`Domain`] can be initialized using either [`Domain::new`] or `TryFrom<&str>`.

```
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

Unicode/internationalized labels are allowed, but must be Puny-encodable and not contain any conflicting bidirectionality constraints. [`Domain`] will encode such labels using [Punycode](https://en.wikipedia.org/wiki/Punycode) when it finds them, ensuring the resulting hostname will always be ASCII-only.

Post-parsing, [`Domain`] gives you access to each individual component, or the whole thing:

```
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
*/

#![forbid(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::redundant_pub_crate,
)]

#![cfg_attr(docsrs, feature(doc_cfg))]



mod idna;
mod psl;
mod puny;



use idna::{
	CharKind,
	IdnaChars,
};
use psl::SuffixKind;
use std::{
	cmp::Ordering,
	fmt,
	hash::{
		Hash,
		Hasher,
	},
	io::{
		Error,
		ErrorKind,
	},
	ops::{
		Deref,
		Range,
	},
	str::FromStr,
};
use unicode_bidi::{
	bidi_class,
	BidiClass,
};
use unicode_normalization::{
	IsNormalized,
	UnicodeNormalization,
};



/// # Punycode Prefix.
const PREFIX: &str = "xn--";



#[derive(Debug, Default, Clone)]
/// # Domain.
///
/// This struct validates and normalizes Internet hostnames, like
/// "www.domain.com".
///
/// It will:
/// * Validate, normalize, and Puny-encode internationalized/Unicode labels ([RFC 3492](https://datatracker.ietf.org/doc/html/rfc3492#ref-IDNA));
/// * Validate and normalize the [public suffix](https://publicsuffix.org/list/);
/// * Ensure conformance with [RFC 1123](https://datatracker.ietf.org/doc/html/rfc1123);
/// * And locate the boundaries of the subdomain (if any), root (required), and suffix (required);
///
/// Suffix and IDNA reference data is compiled at build-time, allowing for very
/// fast runtime parsing, but at the cost of _temporality_.
///
/// Projects using this library should periodically issue new releases or risk
/// growing stale.
///
/// ## Examples
///
/// New instances can be initialized using either [`Domain::new`] or `TryFrom<&str>`.
///
/// ```
/// use adbyss_psl::Domain;
///
/// // These are equivalent and fine:
/// assert!(Domain::new("www.MyDomain.com").is_some());
/// assert!(Domain::try_from("www.MyDomain.com").is_ok());
///
/// // The following is valid DNS, but invalid as an Internet hostname:
/// assert!(Domain::new("_acme-challenge.mydomain.com").is_none());
/// ```
///
/// Valid Internet hostnames must be no longer than 253 characters, and contain
/// both root and (valid) suffix components.
///
/// Their labels — the bits between the dots — must additionally:
/// * Be no longer than 63 characters;
/// * (Ultimately) contain only ASCII letters, digits, and `-`;
/// * Start and end with an alphanumeric character;
///
/// Unicode/internationalized labels are allowed, but must be Puny-encodable
/// and not contain any conflicting bidirectionality constraints. [`Domain`]
/// will encode such labels using [Punycode](https://en.wikipedia.org/wiki/Punycode)
/// when it finds them, ensuring the resulting hostname will always be
/// lowercase ASCII.
pub struct Domain {
	host: String,
	root: Range<usize>,
	suffix: Range<usize>,
}

impl AsRef<str> for Domain {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl AsRef<[u8]> for Domain {
	#[inline]
	fn as_ref(&self) -> &[u8] { self.as_bytes() }
}

impl Deref for Domain {
	type Target = str;
	#[inline]
	fn deref(&self) -> &Self::Target { &self.host }
}

impl Eq for Domain {}

impl fmt::Display for Domain {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for Domain {
	type Err = Error;
	fn from_str(src: &str) -> Result<Self, Self::Err> {
		Self::new(src).ok_or_else(|| ErrorKind::InvalidData.into())
	}
}

impl Hash for Domain {
	#[inline]
	fn hash<H: Hasher>(&self, state: &mut H) { self.host.hash(state); }
}

impl Ord for Domain {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { self.host.cmp(&other.host) }
}

impl PartialEq for Domain {
	#[inline]
	fn eq(&self, other: &Self) -> bool { self.host == other.host }
}

macro_rules! partial_eq {
	// Dereference.
	(deref: $($cast:ident $ty:ty),+ $(,)?) => ($(
		impl PartialEq<$ty> for Domain {
			#[inline]
			fn eq(&self, other: &$ty) -> bool { self.$cast() == *other }
		}

		impl PartialEq<Domain> for $ty {
			#[inline]
			fn eq(&self, other: &Domain) -> bool { other.$cast() == *self }
		}
	)+);

	// Plain.
	($($cast:ident $ty:ty),+ $(,)?) => ($(
		impl PartialEq<$ty> for Domain {
			#[inline]
			fn eq(&self, other: &$ty) -> bool { self.$cast() == other }
		}

		impl PartialEq<Domain> for $ty {
			#[inline]
			fn eq(&self, other: &Domain) -> bool { other.$cast() == self }
		}
	)+);
}

partial_eq!(
	as_str str,
	as_str String,
);

partial_eq!(
	deref:
	as_str &str,
	as_str &String,
);

impl PartialOrd for Domain {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

macro_rules! impl_try {
	($($ty:ty),+) => ($(
		impl TryFrom<$ty> for Domain {
			type Error = Error;
			fn try_from(src: $ty) -> Result<Self, Self::Error> {
				Self::new(src).ok_or_else(|| ErrorKind::InvalidData.into())
			}
		}
	)+)
}

// Aliases for Domain::new.
impl_try!(&str, String, &String);

/// # Main.
impl Domain {
	#[must_use]
	/// # Is Empty.
	pub fn is_empty(&self) -> bool { self.host.is_empty() }

	#[must_use]
	/// # Length.
	pub fn len(&self) -> usize { self.host.len() }

	#[must_use]
	/// # As String Slice.
	pub fn as_str(&self) -> &str { &self.host }

	#[must_use]
	/// # As Bytes.
	pub fn as_bytes(&self) -> &[u8] { self.host.as_bytes() }
}

/// # Setters.
impl Domain {
	/// # New Domain.
	///
	/// Try to parse a given Internet hostname.
	///
	/// Valid Internet hostnames must be no longer than 253 characters, and
	/// contain both root and (valid) suffix components.
	///
	/// Their labels — the bits between the dots — must additionally:
	/// * Be no longer than 63 characters;
	/// * (Ultimately) contain only ASCII letters, digits, and `-`;
	/// * Start and end with an alphanumeric character;
	///
	/// Unicode/internationalized labels are allowed, but must be Puny-encodable
	/// and not contain any conflicting bidirectionality constraints. [`Domain`]
	/// will encode such labels using [Punycode](https://en.wikipedia.org/wiki/Punycode)
	/// when it finds them, ensuring the resulting hostname will always be
	/// lowercase ASCII.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// // A regular ASCII domain:
	/// let dom = Domain::new("www.MyDomain.com").unwrap();
	/// assert_eq!(dom.as_str(), "www.mydomain.com");
	///
	/// // Non-ASCII domains are normalized to Punycode for consistency:
	/// let dom = Domain::new("www.♥.com").unwrap();
	/// assert_eq!(dom.as_str(), "www.xn--g6h.com");
	///
	/// // An incorrectly structured "host" won't parse:
	/// assert!(Domain::new("not.a.domain.123").is_none());
	/// ```
	pub fn new<S>(src: S) -> Option<Self>
	where S: AsRef<str> {
		idna_to_ascii(src.as_ref())
			.and_then(|host| find_dots(host.as_bytes())
				.map(|(mut d, s)| {
					if 0 < d { d += 1; }
					Self {
						root: d..s - 1,
						suffix: s..host.len(),
						host,
					}
				})
			)
	}
}

/// ## WWW.
impl Domain {
	#[must_use]
	/// # Has Leading WWW.
	///
	/// This will return `true` if the domain begins with "www." _and_ that
	/// "www." is a subdomain. (Those aren't always equivalent!)
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom1 = Domain::new("www.blobfolio.com").unwrap();
	/// assert!(dom1.has_www());
	///
	/// let dom2 = Domain::new("blobfolio.com").unwrap();
	/// assert!(! dom2.has_www());
	/// ```
	pub fn has_www(&self) -> bool {
		self.root.start >= 4 && self.host.starts_with("www.")
	}

	/// # Remove Leading WWW.
	///
	/// Modify the domain in-place to remove the leading WWW subdomain. If
	/// a change is made, `true` is returned, otherwise `false`.
	///
	/// By default, only the first leading "www." is stripped; if `recurse` is
	/// true, it will also strip back-to-back occurrences like those in
	/// `www.www.foobar.com`.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let mut dom = Domain::new("www.www.blobfolio.com").unwrap();
	/// assert_eq!(dom.strip_www(false), true);
	/// assert_eq!(dom, "www.blobfolio.com");
	/// assert_eq!(dom.strip_www(false), true);
	/// assert_eq!(dom, "blobfolio.com");
	/// assert_eq!(dom.strip_www(false), false);
	///
	/// // Recursive stripping in one operation:
	/// let mut dom = Domain::new("www.www.blobfolio.com").unwrap();
	/// assert_eq!(dom.strip_www(true), true);
	/// assert_eq!(dom, "blobfolio.com");
	/// assert_eq!(dom.strip_www(false), false);
	/// ```
	pub fn strip_www(&mut self, recurse: bool) -> bool {
		let mut res: bool = false;
		while self.has_www() {
			// Drop the first four bytes, which we know are "www.".
			self.host.replace_range(..4, "");

			// Adjust the ranges.
			self.root.start -= 4;
			self.root.end -= 4;
			self.suffix.start -= 4;
			self.suffix.end -= 4;

			if ! recurse { return true; }
			res = true;
		}

		res
	}

	#[must_use]
	/// # Clone Without Leading WWW.
	///
	/// This will return a clone of the instance without the leading WWW if it
	/// happens to have one, otherwise `None`.
	///
	/// Note: this only removes the first instance of a WWW subdomain. Use
	/// [`Domain::strip_www`] with the `recurse` flag to fully remove all
	/// leading WWW nonsense.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom1 = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom1, "www.blobfolio.com");
	/// assert_eq!(dom1.without_www().unwrap(), "blobfolio.com");
	///
	/// // This will only strip off the first one.
	/// let dom1 = Domain::new("www.www.blobfolio.com").unwrap();
	/// assert_eq!(dom1, "www.www.blobfolio.com");
	/// assert_eq!(dom1.without_www().unwrap(), "www.blobfolio.com");
	/// ```
	pub fn without_www(&self) -> Option<Self> {
		if self.has_www() {
			let mut new = self.clone();
			new.strip_www(false);
			Some(new)
		}
		else { None }
	}
}

/// # Conversion.
impl Domain {
	#[allow(clippy::missing_const_for_fn)] // Doesn't work.
	#[must_use]
	/// # Take String
	///
	/// Consume the struct, returning the sanitized host as an owned `String`.
	pub fn take(self) -> String { self.host }
}

/// # Getters.
impl Domain {
	#[must_use]
	/// # Host.
	///
	/// Return the sanitized host as a string slice. This is equivalent to
	/// dereferencing the object.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom.host(), "www.blobfolio.com");
	/// ```
	pub fn host(&self) -> &str { &self.host }

	#[must_use]
	/// # Root.
	///
	/// Return the root portion of the host, if any. This does not include any
	/// leading or trailing periods.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom.root(), "blobfolio");
	/// ```
	pub fn root(&self) -> &str {
		&self.host[self.root.start..self.root.end]
	}

	#[must_use]
	/// # Subdomain(s).
	///
	/// Return the subdomain portion of the host, if any. This does not include
	/// any trailing periods.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom.subdomain(), Some("www"));
	/// ```
	pub fn subdomain(&self) -> Option<&str> {
		if self.root.start > 0 { Some(&self.host[0..self.root.start - 1]) }
		else { None }
	}

	#[must_use]
	/// # Suffix.
	///
	/// Return the suffix of the host. This does not include any leading
	/// periods.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom.suffix(), "com");
	/// ```
	pub fn suffix(&self) -> &str {
		&self.host[self.suffix.start..self.suffix.end]
	}

	#[must_use]
	/// # TLD.
	///
	/// Return the TLD portion of the host, i.e. everything but the
	/// subdomain(s).
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("www.blobfolio.com").unwrap();
	/// assert_eq!(dom.tld(), "blobfolio.com");
	/// ```
	pub fn tld(&self) -> &str { &self.host[self.root.start..] }
}



#[cfg(any(test, feature = "serde"))]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for Domain {
	#[inline]
	/// # Serialize.
	///
	/// Use the optional `serde` crate feature to enable serialization support.
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: serde::Serializer { serializer.serialize_str(&self.host) }
}

#[cfg(any(test, feature = "serde"))]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for Domain {
	/// # Deserialize.
	///
	/// Use the optional `serde` crate feature to enable serialization support.
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: serde::de::Deserializer<'de> {
		struct DomainVisitor;

		impl<'de> serde::de::Visitor<'de> for DomainVisitor {
			type Value = Domain;
			fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
				f.write_str("domain string")
			}

			fn visit_str<S>(self, src: &str) -> Result<Domain, S>
			where S: serde::de::Error {
				Domain::new(src)
					.ok_or_else(|| serde::de::Error::custom("invalid domain"))
			}

			fn visit_bytes<S>(self, src: &[u8]) -> Result<Domain, S>
			where S: serde::de::Error {
				std::str::from_utf8(src)
					.ok()
					.and_then(Domain::new)
					.ok_or_else(|| serde::de::Error::custom("invalid domain"))
			}
		}

		deserializer.deserialize_str(DomainVisitor)
	}
}



/// # Find Dots.
///
/// The hardest part of suffix validation is teasing the suffix out of the
/// hostname. Odd.
///
/// The suffix cannot be the whole of the thing, but should be the biggest
/// matching chunk of the host.
///
/// If a match is found, the location of the start of the root (its dot, or zero)
/// is returned along with the starting index of the suffix (after its dot).
fn find_dots(host: &[u8]) -> Option<(usize, usize)> {
	// We can avoid all this if the host is too short or only consists of a TLD.
	if host.len() < 3 || SuffixKind::from_slice(host).is_some() { return None; }

	let mut last: usize = 0;
	let mut dot: usize = 0;
	for (idx, _) in host.iter().enumerate().filter(|(_, &b)| b'.' == b) {
		if let Some(suffix) = host.get(idx + 1..).and_then(SuffixKind::from_slice) {
			return match suffix {
				SuffixKind::Tld => Some((dot, idx + 1)),
				SuffixKind::Wild =>
					if dot == 0 { None }
					else { Some((last, dot + 1)) },
				SuffixKind::WildEx(ex) => {
					// Our last chunk might start at zero instead of dot-plus-one.
					let after_dot: usize =
						if dot == 0 { 0 }
						else { dot + 1 };

					// This matches a wildcard exception, making the found suffix
					// the true suffix.
					if host.get(after_dot..idx).map_or(false, |h| ex.is_match(h)) {
						Some((dot, idx + 1))
					}
					// There has to be a before-before part.
					else if dot == 0 { None }
					// Otherwise the last chunk is part of the suffix.
					else { Some((last, after_dot)) }
				},
			};
		}

		std::mem::swap(&mut dot, &mut last);
		dot = idx;
	}

	None
}

/// # Domain to ASCII.
///
/// Normalize a domain according to the IDNA/Punycode guidelines, and return
/// the result.
///
/// Note: this does not enforce public suffix rules; that is processed
/// elsewhere.
fn idna_to_ascii(src: &str) -> Option<String> {
	let src: &str = src.trim_matches(|c: char| c == '.' || c.is_ascii_whitespace());
	if src.is_empty() { return None; }

	// Are things looking nice and simple?
	let bytes = src.as_bytes();
	let mut cap: bool = false;
	let mut dot: bool = false;
	let mut dash: bool = false;
	if
		// Not too long.
		bytes.len() < 254 &&
		// Everything is alphanumeric, a dash, or a dot. During the check,
		// we'll check to make sure there is at least one dot, and note
		// whether there are any uppercase characters or dashes, which would
		// require additional checking.
		bytes.iter().all(|&b| match b {
			b'.' => {
				dot = true;
				true
			},
			// Dashes might be fine, but we should leave a note as we'll have
			// to verify a few additional things.
			b'-' => {
				dash = true;
				true
			}
			// We'll ultimately want to return a lowercase host string, so we
			// should make note if there are any characters requiring
			// conversion.
			b'A'..=b'Z' => {
				cap = true;
				true
			},
			b'a'..=b'z' | b'0'..=b'9' => true,
			_ => false,
		}) &&
		// There is at least one dot somewhere in the middle.
		dot &&
		// None of the between-dot chunks are empty or too long, and if there
		// are dashes, they can't be at the start or end, and there can't be
		// two adjacent ones (which might require PUNY verification).
		bytes.split(|b| b'.'.eq(b))
			.all(|chunk|
				! chunk.is_empty() &&
				chunk.len() < 64 &&
				(
					! dash ||
					(
						! chunk.starts_with(b"xn--") &&
						chunk[0] != b'-' &&
						chunk[chunk.len() - 1] != b'-'
					)
				)
			)
	{
		if cap { Some(src.to_ascii_lowercase()) }
		else { Some(src.to_owned()) }
	}
	// Do it the hard way!
	else { idna_to_ascii_slow(src) }
}

/// # To ASCII (Slow).
///
/// This method is called by [`to_ascii`] when a string is too complicated to
/// verify on-the-fly.
fn idna_to_ascii_slow(src: &str) -> Option<String> {
	// Walk through the string character by character, mapping and normalizing
	// as we go.
	let mut error: bool = false;
	let iter = IdnaChars::new(src, &mut error).nfc();

	// Suck it into a string buffer, but also note whether we have any
	// instances of PUNY prefixes.
	let mut prefix: IdnaPrefix = IdnaPrefix::Dot;
	let mut scratch: Vec<char> = Vec::with_capacity(253);
	for c in iter {
		scratch.push(c);
		prefix = prefix.advance(c);
	}

	// Abort if there was an error, or we've ended up with trailing or leading
	// dots.
	let scratch_len: usize = scratch.len();
	if error || scratch_len == 0 || scratch[0] == '.' || scratch[scratch_len - 1] == '.' {
		return None;
	}

	// If there were no PUNY prefixes anywhere, we can jump straight to
	// building the output string.
	if ! matches!(prefix, IdnaPrefix::Dash2) {
		return idna_normalize_c(&scratch);
	}

	// Otherwise we have to decode and validate each entry first.
	let mut normalized: Vec<char> = Vec::with_capacity(scratch.len());
	if ! idna_normalize_b(&scratch, &mut normalized) {
		return None;
	}

	// Now we can finally build the output.
	let mut scratch = String::with_capacity(normalized.len());
	let mut first = true;
	let mut parts: u8 = 0;
	for part in normalized.split(|c| '.'.eq(c)) {
		if first { first = false; }
		else { scratch.push('.'); }

		// ASCII is nice and easy.
		if part.iter().all(char::is_ascii) { scratch.extend(part); }
		// Unicode requires Punyfication.
		else {
			scratch.push_str(PREFIX);
			if ! puny::encode_into(part, &mut scratch) { return None; }
		}

		parts += 1;
	}

	// One last validation pass.
	if 1 < parts && scratch.len() < 254 { Some(scratch) }
	else { None }
}

#[allow(clippy::similar_names)]
/// BIDI Checks.
///
/// This runs extra checks for any domains containing BIDI control characters.
///
/// See also: <http://tools.ietf.org/html/rfc5893#section-2>
fn idna_check_bidi(part: &[char]) -> bool {
	match bidi_class(part[0]) {
		// LTR.
		BidiClass::L => {
			let mut nom: bool = false;

			// Reverse the iterator; looking from the end makes it easier to
			// check the value of the last non-NSM character.
			for c in part.iter().skip(1).rev().map(|c| bidi_class(*c)) {
				match c {
					BidiClass::NSM => {},
					BidiClass::BN | BidiClass::CS | BidiClass::EN | BidiClass::ES |
					BidiClass::ET | BidiClass::L | BidiClass::ON => if ! nom {
						// The last non-NSM character must be L or EN.
						if c == BidiClass::L || c == BidiClass::EN {
							nom = true;
						}
						else { return false; }
					},
					// Conflicting BIDI.
					_ => return false,
				}
			}

			true
		},

		// RTL.
		BidiClass::R | BidiClass::AL => {
			let mut has_an: bool = false;
			let mut has_en: bool = false;
			let mut nom: bool = false;

			// Reverse the iterator; looking from the end makes it easier to
			// check the value of the last non-NSM character.
			for c in part.iter().skip(1).rev().map(|c| bidi_class(*c)) {
				match c {
					BidiClass::AN => {
						// There cannot be both AN and EN present.
						if has_en { return false; }
						has_an = true;
						nom = true;
					},
					BidiClass::EN => {
						// There cannot be both AN and EN present.
						if has_an { return false; }
						has_en = true;
						nom = true;
					},
					BidiClass::NSM => {},
					BidiClass::AL | BidiClass::BN | BidiClass::CS | BidiClass::ES |
					BidiClass::ET | BidiClass::ON | BidiClass::R => if ! nom {
						// The last non-NSM character must be R, AL, AN, or EN.
						// AN/EN hit a different match ram, so here we're only
						// looking for R or AL.
						if c == BidiClass::R || c == BidiClass::AL {
							nom = true;
						}
						else { return false; }
					},
					// Conflicting BIDI.
					_ => return false,
				}
			}

			true
		},

		// Neither.
		_ => false,
	}
}

/// Check Validity.
///
/// This method checks to ensure the part is not empty, does not begin or end
/// with a dash, does not begin with a combining mark, and does not otherwise
/// contain any restricted characters.
///
/// See also: <http://www.unicode.org/reports/tr46/#Validity_Criteria>
fn idna_check_validity(part: &[char], deep: bool) -> bool {
	let len: usize = part.len();

	0 < len &&
	len < 64 &&
	part[0] != '-' &&
	part[len - 1] != '-' &&
	! unicode_normalization::char::is_combining_mark(part[0]) &&
	(! deep || part.iter().copied().all(CharKind::is_valid))
}

/// # Has BIDI?
///
/// This method checks for the presence of BIDI control characters.
fn idna_has_bidi(part: &[char]) -> bool {
	part.iter()
		.copied()
		.any(|c|
			! c.is_ascii_graphic() &&
			matches!(bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::AN)
		)
}

/// # Normalize Domain (B).
///
/// This pass checks each part of a domain, decoding any PUNY it finds, and
/// ensures each part passes all the rules it's supposed to pass.
///
/// See also: <http://www.unicode.org/reports/tr46/#Processing>
fn idna_normalize_b(src: &[char], out: &mut Vec<char>) -> bool {
	let mut first = true;
	let mut is_bidi = false;
	for part in src.split(|c| '.'.eq(c)) {
		// Replace the dot lost in the split.
		if first { first = false; }
		else { out.push('.'); }

		// Handle PUNY chunk.
		if let Some(chunk) = part.strip_prefix(&['x', 'n', '-', '-']) {
			let Some(mut decoded_part) = puny::decode(chunk) else { return false; };

			// Make sure the decoded version didn't introduce anything
			// illegal.
			if ! idna_check_validity(&decoded_part, true) { return false; }

			// We have to make sure the decoded bit is properly NFC.
			match unicode_normalization::is_nfc_quick(decoded_part.iter().copied()) {
				IsNormalized::Yes => {},
				IsNormalized::No => return false,
				IsNormalized::Maybe => {
					if ! decoded_part.iter().copied().eq(decoded_part.iter().copied().nfc()) {
						return false;
					}
				},
			}

			// Check for BIDI again.
			if ! is_bidi && idna_has_bidi(&decoded_part) { is_bidi = true; }

			out.append(&mut decoded_part);
		}
		// Handle normal chunk.
		else {
			// This is already NFC, but might be weird in other ways.
			if ! idna_check_validity(part, false) { return false; }

			// Check for BIDI.
			if ! is_bidi && idna_has_bidi(part) { is_bidi = true; }

			out.extend_from_slice(part);
		}
	}

	// Apply BIDI checks or we're done!
	! is_bidi || out.split(|c| '.'.eq(c)).all(idna_check_bidi)
}

/// # Normalize Domain (C).
///
/// This pass is used when no PUNY decoding is necessary.
fn idna_normalize_c(src: &[char]) -> Option<String> {
	let mut out = String::with_capacity(253);
	let mut first = true;
	let mut parts: u8 = 0;
	let is_bidi: bool = idna_has_bidi(src);
	for part in src.split(|c| '.'.eq(c)) {
		// Replace the dot lost in the split.
		if first { first = false; }
		else { out.push('.'); }

		// This is already NFC, but might be weird in other ways.
		if ! idna_check_validity(part, false) || (is_bidi && ! idna_check_bidi(part)) {
			return None;
		}

		// We can pass it straight through.
		if part.iter().all(char::is_ascii) { out.extend(part); }
		// We have to encode it.
		else {
			out.push_str(PREFIX);
			if ! puny::encode_into(part, &mut out) { return None; }
		}

		parts += 1;
	}

	if 1 < parts && out.len() < 254 { Some(out) }
	else { None }
}



#[repr(u8)]
#[derive(Clone, Copy)]
/// # IDNA Prefix.
///
/// All this does is look for the pattern `b".xn--"` while iterating through a
/// stream of chars. The goal is to discover whether or not it exists at all,
/// so once [`IdnaPrefix::Dash2`] is set, it never goes away.
enum IdnaPrefix {
	Na,
	Dot,
	Ex,
	En,
	Dash1,
	Dash2,
}

impl IdnaPrefix {
	/// # Advance.
	const fn advance(self, ch: char) -> Self {
		match (ch, self) {
			(_, Self::Dash2) | ('-', Self::Dash1) => Self::Dash2,
			('.', _) => Self::Dot,
			('x', Self::Dot) => Self::Ex,
			('n', Self::Ex) => Self::En,
			('-', Self::En) => Self::Dash1,
			_ => Self::Na,
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;
	use brunch as _;

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
			let res = Domain::new(a);
			assert!(
				res.is_none(),
				"Unexpectedly parsed: {:?}\n{:?}\n", a, res
			);
		}
		// We should have a TLD!
		else {
			if let Some(dom) = Domain::new(a) {
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
		let mut dom = Domain::new("abc.www.食狮.中国").unwrap();
		assert_eq!(dom.subdomain(), Some("abc.www"));
		assert_eq!(dom.root(), "xn--85x722f");
		assert_eq!(dom.suffix(), "xn--fiqs8s");
		assert_eq!(dom.tld(), "xn--85x722f.xn--fiqs8s");
		assert_eq!(dom.host(), "abc.www.xn--85x722f.xn--fiqs8s");

		// Make sure dereference does the right thing. It should...
		assert_eq!(dom.host(), dom.deref());

		dom = Domain::new("blobfolio.com").unwrap();
		assert_eq!(dom.subdomain(), None);
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.host(), "blobfolio.com");

		dom = Domain::new("www.blobfolio.com").unwrap();
		assert_eq!(dom.subdomain(), Some("www"));
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.host(), "www.blobfolio.com");

		// Test a long subdomain.
		dom = Domain::new("another.damn.sub.domain.blobfolio.com").unwrap();
		assert_eq!(dom.subdomain(), Some("another.damn.sub.domain"));
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.host(), "another.damn.sub.domain.blobfolio.com");

		// Also make sure stripping works OK.
		dom = Domain::new("    ....blobfolio.com....    ").unwrap();
		assert_eq!(dom.subdomain(), None);
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.host(), "blobfolio.com");
	}

	#[test]
	/// # Test WWW Stripping.
	fn t_without_www() {
		let dom1 = Domain::new("www.blobfolio.com").unwrap();
		assert!(dom1.has_www());

		let dom2 = dom1.without_www().unwrap();
		assert_eq!(dom2.subdomain(), None);
		assert_eq!(dom2.root(), "blobfolio");
		assert_eq!(dom2.suffix(), "com");
		assert_eq!(dom2.tld(), "blobfolio.com");
		assert_eq!(dom2.host(), "blobfolio.com");
		assert!(! dom2.has_www());
	}

	#[test]
	/// # Serde tests.
	fn t_serde() {
		let dom1: Domain = Domain::new("serialize.domain.com")
			.expect("Domain failed.");

		// Serialize it.
		let serial: String = serde_json::to_string(&dom1)
			.expect("Serialize failed.");
		assert_eq!(serial, "\"serialize.domain.com\"");

		// Deserialize it.
		let dom2: Domain = serde_json::from_str(&serial).expect("Deserialize failed.");
		assert_eq!(dom1, dom2);
	}

	#[test]
	fn t_idna_valid() {
		assert!(matches!(CharKind::from_char('-'), Some(CharKind::Valid)));
		assert!(matches!(CharKind::from_char('.'), Some(CharKind::Valid)));
		for c in '0'..='9' {
			assert!(matches!(CharKind::from_char(c), Some(CharKind::Valid)));
		}
		for c in 'a'..='z' {
			assert!(matches!(CharKind::from_char(c), Some(CharKind::Valid)));
		}
	}

	#[test]
	fn t_idna() {
		let tests = std::fs::read(concat!(env!("OUT_DIR"), "/adbyss-idna-tests.json"))
			.expect("Missing IDNA/Unicode test data.");
		let tests: Vec<(String, Option<String>)> = serde_json::from_slice(&tests)
			.expect("Failed to parse IDNA/Unicode test data.");
		assert!(! tests.is_empty(), "Failed to parse IDNA/Unicode test data.");
		for (i, o) in tests {
			assert_eq!(
				idna_to_ascii(&i),
				o,
				"IDNA handling failed: {:?}", i
			);
		}
	}
}
