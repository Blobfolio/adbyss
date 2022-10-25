/*!
# Adbyss: IDNA
*/

use std::{
	cmp::Ordering,
	num::NonZeroU32,
	str::Chars,
};

// This is compiled by `build.rs` using the template `../skel/idna.rs.txt`. It
// brings in:
// * `fn map_get(ch: u32) -> Option<CharKind>`
// * `static MAP_STR: [char]`
// * `static MAP: [(u32, Option<NonZeroU32>, CharKind)]`
include!(concat!(env!("OUT_DIR"), "/adbyss-idna.rs"));



#[repr(u8)]
#[derive(Clone, Copy)]
/// # Char Kinds.
///
/// This is the IDNA/Unicode status for a given character:
/// * [`CharKind::Valid`]: The character is fine as-is.
/// * [`CharKind::Ignored`]: The character should be silently skipped.
/// * [`CharKind::Mapped`]: The character should be transformed into one or more alternative characters.
pub(super) enum CharKind {
	Valid,
	Ignored,
	Mapped(u8, u8, u8),
}

impl CharKind {
	/// # From Char.
	///
	/// Find the status associated with a given character. `None` implies the
	/// character is invalid.
	pub(super) fn from_char(ch: char) -> Option<Self> {
		if let '-'..='.' | '0'..='9' | 'a'..='z' = ch { Some(Self::Valid) }
		else { map_get(ch as u32) }
	}

	#[inline]
	/// # Is Valid?
	pub(super) fn is_valid(ch: char) -> bool {
		matches!(Self::from_char(ch), Some(Self::Valid))
	}
}



/// # IDNA Character Walker.
///
/// This is a very crude character traversal iterator that checks for character
/// legality and applies any mapping substitutions as needed before yielding.
///
/// This is an iterator rather than a collector to take advantage of the
/// `UnicodeNormalization::nfc` trait.
///
/// The internal `error` field holds a reference to a shared error state, so
/// that afterwards it can be known whether or not the process actually
/// finished correctly.
pub(super) struct IdnaChars<'a> {
	chars: Chars<'a>,
	remap: Option<(usize, u8)>,
	error: &'a mut bool,
}

impl<'a> IdnaChars<'a> {
	/// # New!
	pub(super) fn new(src: &'a str, error: &'a mut bool) -> Self {
		Self {
			chars: src.chars(),
			remap: None,
			error,
		}
	}
}

impl<'a> Iterator for IdnaChars<'a> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		// Read from the mapping slice first, if present.
		if let Some((pos, len)) = &mut self.remap {
			let ch = MAP_STR[*pos];
			if *len > 1 {
				*pos += 1;
				*len -= 1;
			}
			else { self.remap = None; }
			return Some(ch);
		}

		let ch = self.chars.next()?;

		// Short-circuit standard alphanumeric inputs that are totally fine.
		if let '-'..='.' | '0'..='9' | 'a'..='z' = ch { return Some(ch); }

		// Otherwise determine the char/status from the terrible mapping table.
		match CharKind::from_char(ch) {
			Some(CharKind::Valid) => Some(ch),
			Some(CharKind::Mapped(a, b, l)) => {
				let pos = u16::from_le_bytes([a, b]) as usize;
				let ch = MAP_STR[pos];
				if l > 1 { self.remap.replace((pos + 1, l - 1)); }
				Some(ch)
			},
			Some(CharKind::Ignored) => self.next(),
			None => {
				*self.error = true;
				None
			},
		}
	}
}
