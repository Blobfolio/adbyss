/*!
# Adbyss: Public Suffix

This crate provides a very simple interface for checking hosts — ASCII and internationalized — against the [Public Suffix List](https://publicsuffix.org/list/).

This is a judgey library; hosts with unknown or missing suffixes are not parsed. No distinction is made between ICANN and private entries. Rules must be followed! Haha.

For hosts that do get parsed, their values will be normalized to lowercase ASCII.

Note: The suffix reference data is baked into this crate at build time. This reduces the runtime overhead of parsing all that data out, but can also cause implementing apps to grow stale if they haven't been (re)packaged in a while.



## Examples

Initiate a new instance using [`Domain::new`]. If that works, you then have accesses to the individual components:

```
use adbyss_psl::Domain;

// Use `Domain::new()` or `Domain::try_from()` to get started.
let dom = Domain::new("www.MyDomain.com").unwrap();
let dom = Domain::try_from("www.MyDomain.com").unwrap();

// Pull out the pieces if you're into that sort of thing.
assert_eq!(dom.host(), "www.mydomain.com");
assert_eq!(dom.subdomain(), Some("www"));
assert_eq!(dom.root(), "mydomain");
assert_eq!(dom.suffix(), "com");
assert_eq!(dom.tld(), "mydomain.com");

// If you just want the sanitized host back as an owned value, use `Domain::take`:
let owned = dom.take(); // "www.mydomain.com"
```

A [`Domain`] object can be dereferenced to a string slice representing the sanitized host. You can also consume the object into an owned string with [`Domain::take`].



## Optional Crate Features

* `serde`: Enables serialization/deserialization support.
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

#![allow(clippy::module_name_repetitions)]



mod idna;
mod psl;
mod puny;



use idna::CharKind;
use psl::Suffix;
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
	str::Chars,
};
use unicode_bidi::{
	bidi_class,
	BidiClass,
};
use unicode_normalization::UnicodeNormalization;



/// # Punycode Prefix.
const PREFIX: &str = "xn--";



#[derive(Debug, Default, Clone)]
/// # Domain.
///
/// This struct can be used to validate a domain against the [Public Suffix List](https://publicsuffix.org/list/)
/// and separate out subdomain/root/suffix components.
///
/// All valid entries are normalized to lowercase ASCII.
///
/// Note: this is judgey; hosts with unknown or missing suffixes will not parse.
///
/// ## Examples
///
/// Initiate a new instance using [`Domain::new`]. If that works, you then
/// have accesses to the individual components:
///
/// ```
/// use adbyss_psl::Domain;
///
/// // Use `Domain::new()` or `Domain::try_from()` to get started.
/// let dom = Domain::new("www.MyDomain.com").unwrap();
/// let dom = Domain::try_from("www.MyDomain.com").unwrap();
///
/// // Pull out the pieces if you're into that sort of thing.
/// assert_eq!(dom.host(), "www.mydomain.com");
/// assert_eq!(dom.subdomain(), Some("www"));
/// assert_eq!(dom.root(), "mydomain");
/// assert_eq!(dom.suffix(), "com");
/// assert_eq!(dom.tld(), "mydomain.com");
///
/// // If you just want the sanitized host back as an owned value, use
/// // `Domain::take`:
/// let owned = dom.take(); // "www.mydomain.com"
/// ```
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
	/// Try to parse a given host. If the result has both a (valid) suffix and
	/// a root chunk (i.e. it has a TLD), a `Domain` object will be returned.
	///
	/// Hosts with unknown or missing suffixes are rejected. Otherwise all
	/// values are normalized to lowercase ASCII.
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

	#[deprecated(since = "0.5.5", note = "Use Domain::new instead.")]
	#[inline]
	/// # Parse Host.
	///
	/// Alias for [`Domain::new`].
	pub fn parse<S>(src: S) -> Option<Self>
	where S: AsRef<str> { Self::new(src) }
}

/// ## WWW.
impl Domain {
	#[must_use]
	/// # Has Leading WWW.
	///
	/// This will return `true` if the domain begins with "www." _and_ that
	/// "www." is a subdomain. (The latter is usually but not always the case!)
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
			// Chop the string. We know the byte slice starts with "www.", so
			// it should be perfectly safe to shift the pointers down four
			// slots.
			{
				let v = unsafe { self.host.as_mut_vec() };
				let len: usize = v.len() - 4;
				unsafe {
					std::ptr::copy(v.as_ptr().add(4), v.as_mut_ptr(), len);
				}
				v.truncate(len);
			}

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
impl serde::Serialize for Domain {
	#[inline]
	/// # Serialize.
	///
	/// Use the optional `serde` crate feature to enable serialization support.
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: serde::Serializer { serializer.serialize_str(&self.host) }
}

#[cfg(any(test, feature = "serde"))]
impl<'de> serde::Deserialize<'de> for Domain {
	/// # Deserialize.
	///
	/// Use the optional `serde` crate feature to enable serialization support.
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: serde::de::Deserializer<'de> {
		let s: std::borrow::Cow<str> = serde::de::Deserialize::deserialize(deserializer)?;
		Self::new(s).ok_or_else(|| serde::de::Error::custom("Invalid domain."))
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
	if host.len() < 3 || Suffix::from_slice(host).is_some() { return None; }

	let mut last: usize = 0;
	let mut dot: usize = 0;
	for (idx, _) in host.iter().enumerate().filter(|(_, &b)| b'.' == b) {
		if let Some(suffix) = Suffix::from_slice(unsafe { host.get_unchecked(idx + 1..) }) {
			// Wildcard suffixes might have additional requirements.
			if suffix.is_wild() {
				// Our last chunk might start at zero instead of dot-plus-one.
				let after_dot: usize =
					if dot == 0 { 0 }
					else { dot + 1 };

				// This matches a wildcard exception, making the found suffix
				// the true suffix. Note: there cannot be a dot at position
				// zero, so the range is always valid.
				if suffix.wild_match(unsafe { host.get_unchecked(after_dot..idx) }) {
					return Some((dot, idx + 1));
				}

				// There has to be a before-before part.
				if dot == 0 { return None; }

				// Otherwise the last chunk is part of the suffix.
				return Some((last, after_dot));
			}

			// The suffix is what it is.
			return Some((dot, idx + 1));
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
		// require aditional checking.
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
				(! dash || (! chunk.starts_with(b"-") && ! chunk.ends_with(b"-")))
			) &&
		// This isn't PUNY-encoded.
		(! dash || ! src.contains("xn--"))
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
	// Keep two string buffers. We'll switch back and forth as we process the
	// string.
	let mut scratch = String::with_capacity(src.len());
	let mut normalized = String::with_capacity(src.len());

	// Normalize the unicode.
	if ! idna_normalize(src, &mut scratch, &mut normalized) {
		return None;
	}

	// Reset the scratch buffer to store our ASCIIfied output.
	scratch.truncate(0);
	let mut first = true;
	let mut parts: u8 = 0;
	for label in normalized.split('.') {
		parts += 1;
		if first { first = false; }
		else { scratch.push('.'); }

		// ASCII is nice and easy.
		if label.is_ascii() { scratch.push_str(label); }
		// Unicode requires Punyfication.
		else {
			let start = scratch.len();
			scratch.push_str(PREFIX);
			if ! puny::encode_into(&label.chars(), &mut scratch) || scratch.len() - start > 63 {
				return None;
			}
		}
	}

	// One last validation pass.
	if parts < 2 || scratch.len() > 253 {
		return None;
	}

	Some(scratch)
}

#[allow(clippy::similar_names)]
/// BIDI Checks.
///
/// This runs extra checks for any domains containing BIDI control characters.
///
/// See also: <http://tools.ietf.org/html/rfc5893#section-2>
fn idna_check_bidi(label: &str) -> bool {
	let mut chars = label.chars();

	// Check the first character.
	let first_char_class = match chars.next() {
		Some(c) => bidi_class(c),
		None => return true,
	};

	match first_char_class {
		// LTR label
		BidiClass::L => {
			// Rule 5
			if chars.any(idna_check_bidi_ltr) {
				return false;
			}

			// Rule 6
			// must end in L or EN followed by 0 or more NSM
			let mut rev_chars = label.chars().rev();
			let mut last_non_nsm = rev_chars.next();
			loop {
				match last_non_nsm {
					Some(c) if bidi_class(c) == BidiClass::NSM => {
						last_non_nsm = rev_chars.next();
						continue;
					},
					_ => break,
				}
			}
			match last_non_nsm {
				Some(c) if matches!(bidi_class(c), BidiClass::L | BidiClass::EN) => {},
				Some(_) => return false,
				_ => {},
			}
		},

		// RTL label
		BidiClass::R | BidiClass::AL => {
			let mut found_en = false;
			let mut found_an = false;

			// Rule 2
			for c in chars {
				match bidi_class(c) {
					BidiClass::EN => {
						found_en = true;
						if found_an { return false; }
					},
					BidiClass::AN => {
						found_an = true;
						if found_en { return false; }
					},
					BidiClass::AL | BidiClass::BN | BidiClass::CS | BidiClass::ES |
					BidiClass::ET | BidiClass::NSM | BidiClass::ON | BidiClass::R => {},
					_ => return false,
				}
			}

			// Rule 3
			let mut rev_chars = label.chars().rev();
			let mut last = rev_chars.next();
			loop {
				// must end in L or EN followed by 0 or more NSM
				match last {
					Some(c) if bidi_class(c) == BidiClass::NSM => {
						last = rev_chars.next();
						continue;
					},
					_ => break,
				}
			}
			match last {
				Some(c) if matches!(
					bidi_class(c),
					BidiClass::R | BidiClass::AL | BidiClass::EN | BidiClass::AN
				) => {},
				_ => return false,
			}
		},

		// Rule 1: Should start with L or R/AL
		_ => return false,
	}

	true
}

#[inline]
/// # LTR BIDI Invalid Characters.
///
/// Make sure no conflicting BIDI characters are present.
fn idna_check_bidi_ltr(ch: char) -> bool {
	! matches!(
		bidi_class(ch),
		BidiClass::BN | BidiClass::CS | BidiClass::EN | BidiClass::ES |
		BidiClass::ET | BidiClass::L | BidiClass::NSM | BidiClass::ON
	)
}

/// Check Validity.
///
/// This method checks to ensure the part is not empty, does not begin or end
/// with a dash, does not begin with a combining mark, and does not otherwise
/// contain any restricted characters.
///
/// See also: <http://www.unicode.org/reports/tr46/#Validity_Criteria>
fn idna_check_validity(label: &str, deep: bool) -> bool {
	let first_char = match label.chars().next() {
		Some(ch) => ch,
		None => return false,
	};

	! (
		label.starts_with('-') ||
		label.ends_with('-') ||
		unicode_normalization::char::is_combining_mark(first_char)
	) &&
	(
		! deep ||
		label.chars().all(|c| matches!(CharKind::from_char(c), Some(CharKind::Valid)))
	)
}

/// # Has BIDI?
///
/// This method checks for the presence of BIDI control characters.
fn idna_has_bidi(s: &str) -> bool {
	s.chars()
		.any(|c|
			! c.is_ascii_graphic() &&
			matches!(bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::AN)
		)
}

/// # Normalize Domain.
///
/// This is a preprocessing stage that gives us a level playing field to work
/// from. This method also catches most formatting errors, returning `false` if
/// the domain is invalid.
///
/// See also: <http://www.unicode.org/reports/tr46/#Processing>
fn idna_normalize(src: &str, normalized: &mut String, output: &mut String) -> bool {
	// Normalize the characters.
	let mut err: bool = false;
	normalized.extend(
		IdnaChars {
			chars: src.chars(),
			slice: None,
			err: &mut err,
		}
		.nfc()
	);
	if err || normalized.starts_with('.') || normalized.ends_with('.') { return false; }

	// We'll probably need to deal with puny.
	let mut decoder = puny::Decoder::default();
	let mut first = true;
	let mut is_bidi = false;
	for label in normalized.split('.') {
		// Replace the dot lost in the split.
		if first { first = false; }
		else { output.push('.'); }

		// Handle PUNY chunk.
		if let Some(chunk) = label.strip_prefix(PREFIX) {
			let decode = match decoder.decode(chunk) {
				Some(d) => d,
				None => return false,
			};
			let start = output.len();
			output.extend(decode);
			let decoded_label = &output[start..];

			// Check for BIDI again.
			if ! is_bidi && idna_has_bidi(decoded_label) { is_bidi = true; }

			// Make sure the decoded version didn't introduce anything
			// illegal.
			if
				! unicode_normalization::is_nfc(decoded_label) ||
				! idna_check_validity(decoded_label, true)
			{
				return false;
			}
		}
		// Handle normal chunk.
		else {
			// Can't be too big or too small.
			if label.len() > 63 { return false; }

			// Check for BIDI.
			if ! is_bidi && idna_has_bidi(label) { is_bidi = true; }

			// This is already NFC, but might be weird in other ways.
			if ! idna_check_validity(label, false) { return false; }

			output.push_str(label);
		}
	}

	// Apply BIDI checks or we're done!
	! is_bidi || output.split('.').all(idna_check_bidi)
}



/// # IDNA Character Walker.
///
/// This is a very crude character traversal iterator that checks for character
/// legality and applies any mapping substitutions as needed before yielding.
///
/// This is an interator rather than a collector to take advantage of the
/// `UnicodeNormalization::nfc` trait.
///
/// The internal `err` field holds a reference to a shared error state, so that
/// afterwards it can be known whether or not the process actually finished.
struct IdnaChars<'a> {
	chars: Chars<'a>,
	slice: Option<Chars<'static>>,
	err: &'a mut bool,
}

impl<'a> Iterator for IdnaChars<'a> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		// This shouldn't happen, but just in case.
		if *self.err { return None; }

		// Read from the mapping slice first, if present.
		if let Some(s) = &mut self.slice {
			if let Some(c) = s.next() { return Some(c); }
			self.slice = None;
		}

		let ch = self.chars.next()?;

		// Short-circuit standard alphanumeric inputs that are totally fine.
		if let '.' | '-' | 'a'..='z' | '0'..='9' = ch {
			return Some(ch);
		}

		// Otherwise determine the char/status from the terrible mapping table.
		match CharKind::from_char(ch) {
			Some(CharKind::Valid) => Some(ch),
			Some(CharKind::Ignored) => self.next(),
			Some(CharKind::Mapped(idx)) => {
				self.slice.replace(idx.as_str().chars());
				self.next()
			},
			None => {
				*self.err = true;
				None
			},
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

		// Check YAML, which is a bit less robust. First from the serial JSON.
		let dom2: Domain = serde_yaml::from_str(&serial).expect("Deserialize failed.");
		assert_eq!(dom1, dom2);

		// Re-serialize in YAML format, which is a bit different.
		let serial: String = serde_yaml::to_string(&dom1)
			.expect("Serialize failed.");
		assert_eq!(serial.trim(), "---\nserialize.domain.com");

		// Deserialize once more.
		let dom2: Domain = serde_yaml::from_str(&serial).expect("Deserialize failed.");
		assert_eq!(dom1, dom2);
	}

	#[test]
	fn t_idna_valid() {
		assert!(matches!(CharKind::from_char('-'), Some(CharKind::Valid)));
		assert!(matches!(CharKind::from_char('.'), Some(CharKind::Valid)));
		for c in ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
			assert!(matches!(CharKind::from_char(c), Some(CharKind::Valid)));
		}
		for c in [
			'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
			'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
		] {
			assert!(matches!(CharKind::from_char(c), Some(CharKind::Valid)));
		}
	}

	#[test]
	fn t_idna() {
		let raw = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/adbyss-idna-tests.rs"))
			.expect("Missing IDNA unit tests.");
		for line in raw.lines() {
			let mut split = line.split_ascii_whitespace();
			if let Some(input) = split.next() {
				let output = split.next().map(String::from);
				assert_eq!(idna_to_ascii(input), output, "Translation failed with input: {:?}", input);
			}
		}
	}
}
