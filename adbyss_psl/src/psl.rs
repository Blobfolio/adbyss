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
	Tld,
	Wild,
	WildEx(WildKind),
}

impl SuffixKind {
	#[inline]
	/// # Suffix Kind From Slice.
	///
	/// Match a suffix from a byte slice, e.g. `b"com"`.
	pub(super) fn from_slice(src: &[u8]) -> Option<Self> {
		if src == b"com" || src == b"net" || src == b"org" { Some(Self::Tld) }
		else { map_get(src) }
	}
}



/// # Map Search.
///
/// Look up an entry in the map and return its value if found. This ultimately
/// just uses a binary search.
///
/// Internally, two static arrays are used to hold this data:
/// * `MAP_K`: all the pre-hashed `u64` keys (TLDs).
/// * `MAP_V`: the `SuffixKind`s associated with the keys.
///
/// Both arrays are ordered the same way.
fn map_get(src: &[u8]) -> Option<SuffixKind> {
	let src: u64 = ahash::RandomState::with_seeds(13, 19, 23, 71).hash_one(src);
	MAP_K.binary_search(&src).ok().map(|idx| MAP_V[idx])
}
