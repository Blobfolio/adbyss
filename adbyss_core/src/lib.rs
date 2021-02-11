/*!
# `Adbyss`: The Hard Bits
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
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



mod error;
mod hosts;
mod sources;

pub use error::AdbyssError;
pub use hosts::Shitlist;
pub use sources::Source;



/// # Flag: All Sources.
///
/// This flag enables all shitlist sources.
pub const FLAG_ALL: u8         = 0b0000_1111;

/// # Flag: `AdAway`.
///
/// This flag enables the `AdAway` shitlist.
pub const FLAG_ADAWAY: u8      = 0b0000_0001;

/// # Flag: `Adbyss`.
///
/// This flag enables `Adbyss`' internal shitlist.
pub const FLAG_ADBYSS: u8      = 0b0000_0010;

/// # Flag: `Steven Black`.
///
/// This flag enables the `Steven Black` shitlist.
pub const FLAG_STEVENBLACK: u8 = 0b0000_0100;

/// # Flag: `Yoyo`.
///
/// This flag enables the `Yoyo` shitlist.
pub const FLAG_YOYO: u8        = 0b0000_1000;

/// # Flag: Backup Before Writing.
///
/// When writing to an existing file, a backup of the original will be made
/// first.
pub const FLAG_BACKUP: u8      = 0b0001_0000;

/// # Flag: Compact Output.
///
/// Group subdomains by their top-level domain, reducing the total number of
/// lines written to the hostfile (as well as its overall disk size).
pub const FLAG_COMPACT: u8     = 0b0010_0000;

/// # Flag: Non-Interactive Mode.
///
/// This flag bypasses the confirmation when writing to an existing file.
pub const FLAG_Y: u8           = 0b0100_0000;

/// # Maximum Host Line.
///
/// The true limit is `256`; this adds a little padding for `0.0.0.0` and
/// whitespace.
pub(crate) const MAX_LINE: usize = 245;



#[cfg(test)]
mod tests {
	use adbyss_psl::Domain;
	use smartstring::{
		LazyCompact,
		SmartString,
	};



	/// # Sanitize Domain.
	///
	/// This ensures the domain is correctly formatted and has a recognized TLD.
	fn sanitize_domain<S>(dom: S) -> Option<SmartString<LazyCompact>>
	where S: AsRef<str> {
		Domain::parse(dom).map(adbyss_psl::Domain::take)
	}

	#[test]
	fn t_sanitize_domain() {
		for (a, b) in [
			("Blobfolio.com", Some(SmartString::<LazyCompact>::from("blobfolio.com"))),
			("www.Blobfolio.com", Some(SmartString::<LazyCompact>::from("www.blobfolio.com"))),
			(" www.Blobfolio.com", Some(SmartString::<LazyCompact>::from("www.blobfolio.com"))),
			("http://www.Blobfolio.com", None),
			("hello", None),
			("localhost", None),
			("☺.com", Some(SmartString::<LazyCompact>::from("xn--74h.com"))),
			("www.☺.com", Some(SmartString::<LazyCompact>::from("www.xn--74h.com"))),
			("www.xn--74h.com", Some(SmartString::<LazyCompact>::from("www.xn--74h.com"))),
		].iter() {
			assert_eq!(sanitize_domain(a), *b);
		}
	}
}
