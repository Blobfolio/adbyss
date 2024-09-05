/*!
# Adbyss: PSL
*/

// This is compiled by `build.rs` using the template `../skel/psl.rs.txt`. It
// brings in:
// * `static MAP_K: [u64]`
// * `static MAP_V: [SuffixKind]`
// * `enum WildKind`
include!(concat!(env!("OUT_DIR"), "/adbyss-psl.rs"));



#[derive(Clone, Copy)]
/// # Suffix Kinds.
///
/// All valid suffixes will be one of the following:
/// * `Tld`: it is what it is.
/// * `Wild`: it is itself and whatever part appears before it.
/// * `WildEx`: it is itself and whatever part appears before it, unless that part matches an exception.
pub(super) enum SuffixKind {
	/// # Normal TLD.
	Tld,

	/// # Wildcard TLD.
	Wild,

	/// # Wildcard Exception.
	WildEx(WildKind),
}

impl SuffixKind {
	#[inline]
	/// # Suffix Kind From Slice.
	///
	/// Match a suffix from a byte slice, e.g. `b"com"`.
	pub(super) fn from_slice(src: &[u8]) -> Option<Self> {
		if src == b"com" || src == b"net" || src == b"org" { Some(Self::Tld) }
		else {
			// Make sure the compiler understands a key for one is a key for
			// all!
			const { assert!(MAP_K.len() == MAP_V.len(), "BUG: MAP_K and MAP_V have different sizes?!"); }

			let src: u64 = crate::AHASHER.hash_one(src);
			let idx = MAP_K.binary_search(&src).ok()?;
			Some(MAP_V[idx])
		}
	}
}
