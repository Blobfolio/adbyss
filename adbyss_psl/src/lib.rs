/*!
# Adbyss: Public Suffix

[![docs.rs](https://img.shields.io/docsrs/adbyss_psl.svg?style=flat-square&label=docs.rs)](https://docs.rs/adbyss_psl/)
[![changelog](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=changelog&color=9b59b6)](https://github.com/Blobfolio/adbyss/blob/master/adbyss_psl/CHANGELOG.md)<br>
[![crates.io](https://img.shields.io/crates/v/adbyss_psl.svg?style=flat-square&label=crates.io)](https://crates.io/crates/adbyss_psl)
[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/adbyss/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/adbyss/actions)
[![deps.rs](https://deps.rs/crate/adbyss_psl/latest/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/crate/adbyss_psl/)<br>
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
// `Domain::take` or `String::from`:
let owned = dom.take(); // "www.mydomain.com"
```



## Optional Crate Features

* `serde`: Enables serialization/deserialization support.
*/

#![forbid(unsafe_code)]

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]

#![cfg_attr(docsrs, feature(doc_cfg))]

mod psl;

use psl::SuffixKind;
use std::{
	borrow::Cow,
	cmp::Ordering,
	fmt,
	hash,
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



/// # Max Local Part (Email Address) Size.
const MAX_LOCAL: usize = 64;



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
	/// # Host.
	host: String,

	/// # Root Range.
	///
	/// Being the middle part, we can use this to figure out where the
	/// subdomain and suffix parts are, i.e. `..root.start - 1` and
	/// `root.end + 1..` respectively.
	root: Range<usize>,
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
	fn deref(&self) -> &Self::Target { self.host.as_str() }
}

impl Eq for Domain {}

impl fmt::Display for Domain {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<str as fmt::Display>::fmt(self.as_str(), f)
	}
}

impl From<Domain> for String {
	#[inline]
	fn from(src: Domain) -> Self { src.host }
}

impl FromStr for Domain {
	type Err = Error;

	#[inline]
	fn from_str(src: &str) -> Result<Self, Self::Err> {
		Self::new(src).ok_or_else(|| ErrorKind::InvalidData.into())
	}
}

impl hash::Hash for Domain {
	#[inline]
	fn hash<H: hash::Hasher>(&self, state: &mut H) { self.host.hash(state); }
}

impl Ord for Domain {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { self.host.cmp(&other.host) }
}

impl PartialEq for Domain {
	#[inline]
	fn eq(&self, other: &Self) -> bool { self.host == other.host }
}

impl PartialEq<str> for Domain {
	#[inline]
	fn eq(&self, other: &str) -> bool { self.as_str() == other }
}
impl PartialEq<Domain> for str {
	#[inline]
	fn eq(&self, other: &Domain) -> bool { <Domain as PartialEq<Self>>::eq(other, self) }
}

/// # Helper: Reciprocal `PartialEq`.
macro_rules! eq {
	($($ty:ty),+) => ($(
		impl PartialEq<$ty> for Domain {
			#[inline]
			fn eq(&self, other: &$ty) -> bool { <Self as PartialEq<str>>::eq(self, other) }
		}
		impl PartialEq<Domain> for $ty {
			#[inline]
			fn eq(&self, other: &Domain) -> bool { <Domain as PartialEq<str>>::eq(other, self) }
		}
	)+);
}
eq!(&str, &String, String, &Cow<'_, str>, Cow<'_, str>, &Box<str>, Box<str>);

impl PartialOrd for Domain {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

/// # Helper: `TryFrom` impl.
macro_rules! impl_try {
	($($ty:ty),+ $(,)?) => ($(
		impl TryFrom<$ty> for Domain {
			type Error = Error;

			#[inline]
			fn try_from(src: $ty) -> Result<Self, Self::Error> {
				idna_to_ascii_borrowed(src)
					.and_then(Self::from_ascii_string)
					.ok_or_else(|| ErrorKind::InvalidData.into())
			}
		}
	)+)
}
impl_try!(&str, &String);

impl TryFrom<Cow<'_, str>> for Domain {
	type Error = Error;

	#[inline]
	fn try_from(src: Cow<'_, str>) -> Result<Self, Self::Error> {
		match src {
			Cow::Borrowed(s) => idna_to_ascii_borrowed(s),
			Cow::Owned(s) => idna_to_ascii_owned(s),
		}
			.and_then(Self::from_ascii_string)
			.ok_or_else(|| ErrorKind::InvalidData.into())
	}
}

impl TryFrom<String> for Domain {
	type Error = Error;

	#[inline]
	fn try_from(src: String) -> Result<Self, Self::Error> {
		idna_to_ascii_owned(src)
			.and_then(Self::from_ascii_string)
			.ok_or_else(|| ErrorKind::InvalidData.into())
	}
}

/// # Main.
impl Domain {
	#[must_use]
	/// # Is Empty.
	pub const fn is_empty(&self) -> bool { self.host.is_empty() }

	#[must_use]
	/// # Length.
	pub const fn len(&self) -> usize { self.host.len() }

	#[must_use]
	/// # As String Slice.
	///
	/// Return the domain as a string slice.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("NETFLIX.COM").unwrap();
	/// assert_eq!(
	///     dom.as_str(),
	///     "netflix.com",
	/// );
	/// ```
	pub const fn as_str(&self) -> &str { self.host.as_str() }

	#[must_use]
	/// # As Byte Slice.
	///
	/// Return the domain as a byte slice.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom = Domain::new("rugbypass.tv").unwrap();
	/// assert_eq!(
	///     dom.as_bytes(),
	///     b"rugbypass.tv",
	/// );
	/// ```
	pub const fn as_bytes(&self) -> &[u8] { self.host.as_bytes() }
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
	where S: AsRef<str> { Self::try_from(src.as_ref()).ok() }

	#[inline]
	/// # From Processed String.
	///
	/// This method finds the dots in a string that has come from idna-to-ascii
	/// conversion and constructs the final `Domain` object.
	///
	/// It exists mainly to reduce duplicate code and the accidental
	/// inconsistencies that stem from it.
	fn from_ascii_string(host: String) -> Option<Self> {
		let bytes = host.as_bytes();
		let len = bytes.len();

		// Find the suffix.
		let suffix = find_suffix(bytes)?;

		// Find the root (and make sure there is one).
		let root_end = len.checked_sub(suffix.len() + 1)?;
		let root = bytes.iter()
			.copied()
			.take(root_end)
			.rposition(|b| b == b'.')
			.map_or(0, |pos| pos + 1)..root_end;

		// Done!
		Some(Self { host, root })
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
	pub const fn has_www(&self) -> bool {
		self.root.start >= 4 &&
		matches!(self.as_bytes(), [b'w', b'w', b'w', b'.', ..])
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

			res = true;
			if ! recurse { break; }
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
	#[must_use]
	/// # Take String
	///
	/// Consume the struct, returning the sanitized host as an owned `String`.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	///
	/// let dom1: String = Domain::new("KilledByGoogle.com")
	///     .unwrap()
	///     .take();
	///
	/// // String::from works too if you prefer that:
	/// let dom2: String = Domain::new("KilledByGoogle.com")
	///     .unwrap()
	///     .into();
	///
	/// assert_eq!(dom1, dom2);
	/// assert_eq!(dom1, "killedbygoogle.com");
	/// ```
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
	pub const fn host(&self) -> &str { self.host.as_str() }

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
	pub const fn subdomain(&self) -> Option<&str> {
		if let Some(end) = self.root.start.checked_sub(1) {
			let (out, _) = self.host.as_str().split_at(end);
			Some(out)
		}
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
	pub const fn suffix(&self) -> &str {
		let (_ ,out) = self.host.as_str().split_at(self.root.end + 1);
		out
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
	pub const fn tld(&self) -> &str {
		let (_, out) = self.host.as_str().split_at(self.root.start);
		out
	}
}

/// # Miscellaneous.
impl Domain {
	#[must_use]
	/// # Validate/Normalize an Email Address.
	///
	/// This one-shot method leverages the power of [`Domain`] to quickly and
	/// efficiently validate and normalize "regular" Internet email addresses
	/// like "user@domain.com".
	///
	/// For local ("user") parts, it largely follows [RFC 5322](https://datatracker.ietf.org/doc/html/rfc5322),
	/// ensuring values:
	/// * Are between `1..=64` bytes in length;
	/// * Comprise only ASCII alphanumerics and ``! # $ % & ' * + - . / = ? ^ _ ` { | } ~``;
	///   * Exception: UPPERCASE is converted to lowercase, as the gods intended.
	///   * Exception: comments and quotes are unsupported, but may be removed if superfluous.
	/// * Do not contain leading, trailing, or consecutive dots;
	///   * Illegal `.` usage will simply be fixed, though, so don't sweat it!
	///
	/// Note: this method will only allocate if the source requires touch-ups.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	/// use std::borrow::Cow;
	///
	/// // All output will always be ASCII lowercase.
	/// assert_eq!(
	///     Domain::email("Hello@World.COM").as_deref(),
	///     Some("hello@world.com"),
	/// );
	/// assert_eq!(
	///     Domain::email("Princess.Peach@Cat♥.com").as_deref(),
	///     Some("princess.peach@xn--cat-1x5a.com"),
	/// );
	///
	/// // Allocation can be avoided for sources that are valid as-are:
	/// assert!(matches!(
	///     Domain::email(" user@domain.com "), // Edge trimming is free!
	///     Some(Cow::Borrowed("user@domain.com")),
	/// ));
	///
	/// // Invalid or unsupported addresses come up `None`:
	/// assert!(Domain::email("user@localhost").is_none()); // No suffix.
	/// assert!(Domain::email("björk@post.com").is_none()); // Non-ASCII local.
	/// assert!(Domain::email("a[b]c@nope.net").is_none()); // Illegal `[]`.
	/// ```
	pub fn email(src: &str) -> Option<Cow<'_, str>> {
		// Trim the absolute ends.
		let src = src
			.trim_start_matches(|c: char| matches!(c, '.' | '"') || c.is_ascii_whitespace())
			.trim_end_matches(|c: char| c == '.' || c.is_ascii_whitespace());

		// Split on @.
		let (src_local, src_host) = src.split_once('@')?;

		// Trim the "inner ends" of each part and normalize them.
		let nice_local = sanitize_email_local(src_local.trim_end_matches(['.', '"']))?;
		let nice_host = sanitize_email_host(src_host.trim_start_matches('.'))?;

		// If nothing changed, return the source.
		if src_local == nice_local && src_host == nice_host {
			Some(Cow::Borrowed(src))
		}
		// Otherwise build and return a new string.
		else {
			let mut out = nice_local.into_owned();
			out.reserve(1 + nice_host.len());
			out.push('@');
			out.push_str(nice_host.as_ref());
			Some(Cow::Owned(out))
		}
	}

	#[must_use]
	/// # Validate/Normalize Email Address Parts.
	///
	/// Like [`Domain::email`], but the local and host parts are kept
	/// separate and returned as a tuple.
	///
	/// ## Examples
	///
	/// ```
	/// use adbyss_psl::Domain;
	/// use std::borrow::Cow;
	///
	/// let (local, host) = Domain::email_parts("Princess.Peach@Cat♥.com")
	///     .unwrap();
	///
	/// assert_eq!(local, "princess.peach");
	/// assert_eq!(host, "xn--cat-1x5a.com");
	/// ```
	pub fn email_parts(src: &str) -> Option<(Cow<'_, str>, Cow<'_, str>)> {
		// Trim the absolute ends.
		let src = src
			.trim_start_matches(|c: char| matches!(c, '.' | '"') || c.is_ascii_whitespace())
			.trim_end_matches(|c: char| c == '.' || c.is_ascii_whitespace());

		// Split on @.
		let (local, host) = src.split_once('@')?;

		// Trim the "inner ends" of each part and normalize them.
		let local = sanitize_email_local(local.trim_end_matches(['.', '"']))?;
		let host = sanitize_email_host(host.trim_start_matches('.'))?;

		// Done!
		Some((local, host))
	}
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
		/// # Visitor Instance.
		struct DomainVisitor;

		impl serde::de::Visitor<'_> for DomainVisitor {
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



/// # Find Suffix.
///
/// Find and return the largest matching suffix, if any.
///
/// Note this method can potentially return a match for the entire host, which
/// isn't valid for our purposes. The caller (`from_ascii_string`) has to
/// check for that edge case anyway, so it'll fail there if not here.
fn find_suffix(host: &[u8]) -> Option<&[u8]> {
	// We can avoid all this if the host is too short.
	if host.len() < 3 { return None; }

	// Note the parts array is backwards, so `.rev()` gives us biggest to
	// smallest.
	let parts = SuffixKind::suffix_parts(host);
	for (idx, part) in parts.iter().copied().enumerate().rev().skip(1) {
		if let Some(part) = part {
			match SuffixKind::from_parts(part, idx + 1) {
				// Normal TLD, direct hit!
				Some(SuffixKind::Tld) => return Some(part),

				// Wildcards match the earlier (bigger) part.
				Some(SuffixKind::Wild) => return parts[idx + 1],

				// Wildcards with exceptions might match either the current
				// part or the earlier (bigger) one, depending on the value.
				Some(SuffixKind::WildEx(ex)) => {
					// Pull the earlier (bigger) chunk.
					let next = parts[idx + 1]?;

					// If the difference between it and the current (minus the
					// dot) is an exception, the current part *is* the TLD.
					if ex.is_match(&next[..next.len() - part.len() - 1]) {
						return Some(part);
					}
					// Otherwise the earlier (bigger) part is the TLD.
					return Some(next);
				},
				_ => {},
			}
		}
	}

	None
}

/// # Domain to ASCII (Borrowed).
///
/// Normalize a domain according to the IDNA/Punycode guidelines, and return
/// the result.
///
/// Note: this does not enforce public suffix rules; that is processed
/// elsewhere.
fn idna_to_ascii_borrowed(src: &str) -> Option<String> {
	let src: &str = src.trim_matches(|c: char| c == '.' || c.is_ascii_whitespace());
	if src.is_empty() { None }
	else { idna_to_ascii_bytes(src.as_bytes()).map(Cow::into_owned) }
}

/// # Domain to ASCII (Owned).
///
/// Same as the borrowed version, but potentially avoids allocating a new
/// `String` if the source is already a happy one.
fn idna_to_ascii_owned(mut src: String) -> Option<String> {
	use trimothy::TrimMatchesMut;

	src.trim_matches_mut(|c: char| c == '.' || c.is_ascii_whitespace());
	if src.is_empty() { None }
	else {
		let out = idna_to_ascii_bytes(src.as_bytes())?;

		// The original was fine.
		if src == out { Some(src) }
		// Something changed!
		else { Some(out.into_owned()) }
	}
}

#[inline]
/// # Domain to ASCII (Bytes).
///
/// This handles the actual `idna` portion of the conversion. (We do other
/// stuff before and after before a domain can be declared good.)
fn idna_to_ascii_bytes(src: &[u8]) -> Option<Cow<'_, str>> {
	use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};

	// One line'll do it!
	Uts46::new().to_ascii(
		src,
		AsciiDenyList::STD3,
		Hyphens::CheckFirstLast,
		DnsLength::Verify,
	).ok()
}

/// # Sanitize (Email) Host Part.
///
/// This performs the same tasks [`Domain::new`] would, but persists
/// the borrow if possible.
///
/// Note values have been pre-trimmed.
fn sanitize_email_host(src: &str) -> Option<Cow<'_, str>> {
	let host = idna_to_ascii_bytes(src.as_bytes())?;

	// Make sure there's a suffix.
	let bytes = host.as_bytes();
	let suffix = find_suffix(bytes)?;

	// So long as there's a root behind the suffix, we're good!
	if suffix.len() + 1 < bytes.len() { Some(host) }
	else { None }
}

/// # Sanitize (Email) Local Part.
///
/// This ensures the local part contains only valid, lowercase characters, and
/// is between `1..=64` bytes in length, touching up as necessary.
///
/// Note values have been pre-trimmed.
fn sanitize_email_local(raw: &str) -> Option<Cow<'_, str>> {
	// Easy abort.
	if raw.is_empty() { return None; }

	// Since this pass is merely checking for existing validity and all valid
	// characters are ASCII, we can loop through bytes rather than chars.
	let mut checked = 0;
	let mut last = b'.';
	if raw.len() <= MAX_LOCAL {
		for b in raw.bytes() {
			match b {
				// Always good.
				b'!' | b'#'..=b'\'' | b'*' | b'+' | b'-' |
				b'/'..=b'9' | b'='  | b'?' | b'^'..=b'~' => {},

				// Maybe good, but we'll have to dig deeper to know for sure.
				b'A'..=b'Z' | b'(' => break,

				// Good if non-consecutive, maybe fixable if not.
				b'.' => if last == b'.' { break; },

				// Never good.
				_ => return None,
			}
			last = b;
			checked += 1;
		}

		// It was already perfect! Hurray!
		if checked == raw.len() { return Some(Cow::Borrowed(raw)); }
	}

	// Fine! Do it the hard way!
	let (good, maybe) = raw.split_at(checked);
	sanitize_email_local_slow(good, maybe, char::from(last))
}

/// # Sanitize (Email) Local Part (Slow).
///
/// This method takes over from `sanitize_email_local` if/when it realizes
/// allocating touch-ups will be required to see it through.
fn sanitize_email_local_slow(good: &str, maybe: &str, mut last: char) -> Option<Cow<'static, str>> {
	use trimothy::TrimMatchesMut;

	// Start with what we found earlier, if anything.
	let mut address = String::with_capacity(good.len() + maybe.len());
	address.push_str(good);

	// Loop through the remainder char-by-char to see what's what!
	let mut chars = maybe.chars();
	while let Some(c) = chars.next() {
		// Skip comments.
		if last == '(' {
			match c {
				// We don't support inner-quote parsing.
				'"' => return None,

				// A backslash invalidates whatever comes next.
				'\\' => { let _ = chars.next()?; },

				// A closing parenthesis is our ticket out of here!
				')' => {
					last =
						// Pretend the last character was a dot if it was.
						if address.is_empty() || address.ends_with('.') { '.' }
						// Otherwise any non-dot will do.
						else { c };
				},

				// Ignore anything else.
				_ => {},
			}

			continue;
		}

		match c {
			// Unconditionally allowed characters (a-z, 0-9, and non-dot specials).
			'!' | '#'..='\'' | '*' | '+' | '-' | '/'..='9' | '=' | '?' | '^'..='~' => {
				last = c;
				address.push(c);
			},

			// Uppercase ASCII is fine, but requires lowercasing.
			'A'..='Z' => {
				last = c.to_ascii_lowercase();
				address.push(last);
			},

			// Periods are fine, but cannot occur back-to-back.
			'.' => if last != c {
				last = c;
				address.push(c);
			},

			// Start of comment section.
			'(' => { last = c; },

			// Boo.
			_ => return None,
		}
	}

	// Abort if a comment went unclosed or the address is empty.
	if last == '(' || address.is_empty() { return None; }

	// Retrim the end if needed.
	if last == '.' {
		address.trim_end_matches_mut('.');
		if address.is_empty() { return None; }
	}

	// Return it unless we're too big!
	if address.len() <= MAX_LOCAL { Some(Cow::Owned(address)) }
	else { None }
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
				"Unexpectedly parsed: {a:?}\n{res:?}\n",
			);

			// The String impl is slightly different; let's make sure it gives
			// the same result.
			let res2 = Domain::try_from(a.to_owned());
			assert!(
				res2.is_err(),
				"Unexpectedly parsed: {a:?} (string)\n{res2:?}\n",
			);
		}
		// We should have a TLD!
		else if let Some(dom) = Domain::new(a) {
			assert_eq!(
				dom.tld(),
				b.unwrap(),
				"Failed parsing: {dom:?}",
			);

			// Again, the String impl is slightly different.
			let Ok(dom2) = Domain::try_from(a.to_owned()) else {
				panic!("Failed parsing: {a:?} (string)");
			};
			assert_eq!(dom, dom2, "String/str parsing mismatch for {a:?}");
		}
		else {
			panic!("Failed parsing: {a:?}");
		}
	}

	#[test]
	#[expect(clippy::cognitive_complexity, reason = "It is what it is.")]
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
		assert_eq!(dom.host(), &*dom);

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

		// Make sure case is correctly adjusted.
		dom = Domain::new("Www.BlobFolio.cOm").unwrap();
		assert_eq!(dom.subdomain(), Some("www"));
		assert_eq!(dom.root(), "blobfolio");
		assert_eq!(dom.suffix(), "com");
		assert_eq!(dom.tld(), "blobfolio.com");
		assert_eq!(dom.host(), "www.blobfolio.com");

		// This is also a good place to verify the String impl matches.
		assert_eq!(dom, Domain::try_from("Www.BlobFolio.cOm".to_owned()).unwrap());

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
	fn t_email() {
		for (raw, expected) in [
			// Thanks Wikipedia!
			("simple@example.com", Some("simple@example.com")),
			("very.common@example.com", Some("very.common@example.com")),
			("FirstName.LastName@EasierReading.org", Some("firstname.lastname@easierreading.org")),
			("x@example.com", Some("x@example.com")),
			("long.email-address-with-hyphens@and.subdomains.example.com", Some("long.email-address-with-hyphens@and.subdomains.example.com")),
			("user.name+tag+sorting@example.com", Some("user.name+tag+sorting@example.com")),
			("name/surname@example.com", Some("name/surname@example.com")),
			("mailhost!username@example.org", Some("mailhost!username@example.org")),
			("user%example.com@example.org", Some("user%example.com@example.org")),
			("user-@example.org", Some("user-@example.org")),
			("abc.example.com", None),
			("a@b@c@example.com", None),
			(r#"a"b(c)d,e:f;g<h>i[j\k]l@example.com"#, None),
			(r#"just"not"right@example.com"#, None),
			(r#"this is"not\allowed@example.com"#, None),
			(r#"this\ still\"not\\allowed@example.com"#, None),
			("1234567890123456789012345678901234567890123456789012345678901234+x@example.com", None),

			// A few more tests.
			(r#""user"@domain.com"#, Some("user@domain.com")),
			("USER(STUPID\tCOMMENT).@DOMAIN.COM", Some("user@domain.com")),
			("user(unclosed@domain.com", None),
			(r#"user(trailing\@domain.com"#, None),
			(r#"user(escape\)unescape)@domain.com"#, Some("user@domain.com")),
			("user@ac.jp", None), // Invalid TLD.
			("user@食狮.com.cn", Some("user@xn--85x722f.com.cn")),
			("björk@bjork.com", None), // Sorry Björk!
			("cow.(goes).moo@domain.com", Some("cow.moo@domain.com")),
			(r#"cow.("björk").moo@domain.com"#, None), // Inner quote in comment.
			("Princess.Peach@Cat♥.com", Some("princess.peach@xn--cat-1x5a.com")),
		] {
			assert_eq!(
				Domain::email(raw).as_deref(),
				expected,
				"{raw} didn't parse as expected.",
			);

			if let Some(expected) = expected {
				let Some((local, host)) = Domain::email_parts(raw) else {
					panic!("Unable to parse {raw} parts.");
				};
				assert_eq!(format!("{local}@{host}"), expected);
			}
			else {
				assert!(Domain::email_parts(raw).is_none());
			}
		}
	}

	#[test]
	fn t_email_borrowed() {
		// All of these should be borrowable.
		for raw in [
			"simple@example.com",
			"very.common@example.com",
			"x@example.com",
			"long.email-address-with-hyphens@and.subdomains.example.com",
			"name/surname@example.com",
			"mailhost!username@example.org",
			"user%example.com@example.org",
			"user-@example.org",
			"  \"..josh@blobfolio.com..  ", // Edge trimming is free.
		] {
			assert!(matches!(Domain::email(raw), Some(Cow::Borrowed(_))));
			let Some((local, host)) = Domain::email_parts(raw) else {
				panic!("Unable to parse {raw} parts.");
			};
			assert!(matches!(local, Cow::Borrowed(_)));
			assert!(matches!(host, Cow::Borrowed(_)));
		}
	}

	#[test]
	fn t_email_owned() {
		// These are not borrowable.
		for raw in [
			"simple@EXAMPLE.COM",
			"VERY.COMMON@example.com",
			"X@EXAMPLE.COM",
			"LONG.EMAIL-ADDRESS-WITH-HYPHENS@AND.SUBDOMAINS.EXAMPLE.COM",
			"NAME/SURNAME@EXAMPLE.COM",
			"MAILHOST!USERNAME@EXAMPLE.ORG",
			"USER%EXAMPLE.COM@EXAMPLE.ORG",
			"USER-@EXAMPLE.ORG",
			r#""user"@domain.com"#,
			"USER(STUPID\tCOMMENT).@DOMAIN.COM",
			"user@食狮.com.cn",
			"cow.(goes).moo@domain.com",
			r#"cow.(björk).moo@domain.com"#,
			"Princess.Peach@Cat♥.com",
			"josh.@blobfolio.com", // Inner trimming is not.
		] {
			assert!(matches!(Domain::email(raw), Some(Cow::Owned(_))));
		}

		// Parts can go both ways.
		for (raw, b1, b2) in [
			("simple@EXAMPLE.COM", true, false),
			("VERY.COMMON@example.com", false, true),
			("X@EXAMPLE.COM", false, false),
			(r#""user"@domain.com"#, true, true),
			("user@食狮.com.cn", true, false),
			("cow.(goes).moo@domain.com", false, true),
		] {
			let Some((local, host)) = Domain::email_parts(raw) else {
				panic!("Unable to parse {raw} parts.");
			};

			if b1 { assert!(matches!(local, Cow::Borrowed(_))); }
			else { assert!(matches!(local, Cow::Owned(_))); }

			if b2 { assert!(matches!(host, Cow::Borrowed(_))); }
			else { assert!(matches!(host, Cow::Owned(_))); }
		}
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
}
