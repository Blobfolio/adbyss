/*!
# `Adbyss`: The Hard Bits
*/

#![deny(unsafe_code)]

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

#![allow(clippy::module_name_repetitions)]



mod error;
mod hosts;
mod ping;
mod sources;

pub use error::AdbyssError;
pub use hosts::Shitlist;
pub use ping::check_internet;
pub use sources::Source;



/// # (Not) Random State.
///
/// Using a fixed seed value for `AHashSet`/`AHashMap` drops a few dependencies
/// and prevents Valgrind complaining about 64 lingering bytes from the runtime
/// static that would be used otherwise.
///
/// For our purposes, the variability of truly random keys isn't really needed.
pub(crate) const AHASH_STATE: ahash::RandomState = ahash::RandomState::with_seeds(13, 19, 23, 71);

/// # Flag: All Sources.
///
/// This flag enables all shitlist sources.
pub const FLAG_ALL: u8 =         0b0000_1111;

/// # Flag: `AdAway`.
///
/// This flag enables the `AdAway` shitlist.
pub const FLAG_ADAWAY: u8 =      0b0000_0001;

/// # Flag: `Adbyss`.
///
/// This flag enables `Adbyss`' internal shitlist.
pub const FLAG_ADBYSS: u8 =      0b0000_0010;

/// # Flag: `Steven Black`.
///
/// This flag enables the `Steven Black` shitlist.
pub const FLAG_STEVENBLACK: u8 = 0b0000_0100;

/// # Flag: `Yoyo`.
///
/// This flag enables the `Yoyo` shitlist.
pub const FLAG_YOYO: u8 =        0b0000_1000;

/// # Flag: Backup Before Writing.
///
/// When writing to an existing file, a backup of the original will be made
/// first.
pub const FLAG_BACKUP: u8 =      0b0001_0000;

/// # Flag: Compact Output.
///
/// Group subdomains by their top-level domain, reducing the total number of
/// lines written to the hostfile (as well as its overall disk size).
pub const FLAG_COMPACT: u8 =     0b0010_0000;

/// # Flag: Non-Interactive Mode.
///
/// This flag bypasses the confirmation when writing to an existing file.
pub const FLAG_Y: u8 =           0b0100_0000;

/// # Maximum Host Line.
///
/// The true limit is `256`; this adds a little padding for `0.0.0.0` and
/// whitespace.
pub(crate) const MAX_LINE: usize = 245;



#[cfg(test)]
mod tests {
	use adbyss_psl::Domain;



	/// # Sanitize Domain.
	///
	/// This ensures the domain is correctly formatted and has a recognized TLD.
	fn sanitize_domain<S>(dom: S) -> Option<String>
	where S: AsRef<str> {
		Domain::new(dom).map(adbyss_psl::Domain::take)
	}

	#[test]
	fn t_sanitize_domain() {
		for (a, b) in [
			("Blobfolio.com", Some(String::from("blobfolio.com"))),
			("www.Blobfolio.com", Some(String::from("www.blobfolio.com"))),
			(" www.Blobfolio.com", Some(String::from("www.blobfolio.com"))),
			("http://www.Blobfolio.com", None),
			("hello", None),
			("localhost", None),
			("☺.com", Some(String::from("xn--74h.com"))),
			("www.☺.com", Some(String::from("www.xn--74h.com"))),
			("www.xn--74h.com", Some(String::from("www.xn--74h.com"))),
		].iter() {
			assert_eq!(sanitize_domain(a), *b);
		}
	}
}
