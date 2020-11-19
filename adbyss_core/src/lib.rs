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



mod shitlist;
use adbyss_psl::Domain;
pub use shitlist::{
	Shitlist,
	FLAG_ALL,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_STEVENBLACK,
	FLAG_YOYO,
	FLAG_BACKUP,
	FLAG_COMPACT,
	FLAG_Y,
};



#[must_use]
/// # Sanitize Domain.
///
/// This ensures the domain is correctly formatted and has a recognized TLD.
pub fn sanitize_domain<S>(dom: S) -> Option<String>
where S: AsRef<str> {
	Domain::parse(dom).map(adbyss_psl::Domain::take)
}



#[cfg(test)]
mod tests {
	use super::*;

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
