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
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



mod shitlist;
pub use shitlist::{
	Shitlist,
	FLAG_ALL,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_STEVENBLACK,
	FLAG_YOYO,
	FLAG_BACKUP,
	FLAG_FRESH,
	FLAG_Y,
};



#[must_use]
/// # Sanitize Domain.
///
/// This ensures the domain is correctly formatted and has a recognized TLD.
pub fn sanitize_domain(dom: &str) -> Option<String> {
	use publicsuffix::{
		Host,
		List,
	};

	lazy_static::lazy_static! {
		// Load the Public Suffix List only once.
		static ref PSL: List = List::from_str(
			include_str!("../skel/public_suffix_list.dat")
		).expect("Unable to load Public Suffix list.");
	}

	// Look for the domain any which way it happens to be.
	if let Ok(Host::Domain(dom)) = PSL.parse_str(dom.trim()) {
		// It should have a root and a suffix.
		if
			dom.is_icann() &&
			dom.suffix().is_some() &&
			dom.has_known_suffix() &&
			dom.root().is_some()
		{
			let mut domain: String = dom.full().to_lowercase();

			// Handle Unicode-formatted domains.
			if ! domain.is_ascii() {
				match punycode::encode(&domain) {
					Ok(s) => {
						domain = [
							"xn--",
							&s
						].concat()
					},
					Err(_) => return None,
				}
			}

			if ! domain.is_empty() {
				return Some(domain);
			}
		}
	}

	None
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
			("http://www.Blobfolio.com", Some(String::from("www.blobfolio.com"))),
			("hello", None),
			("localhost", None),
		].iter() {
			assert_eq!(sanitize_domain(a), *b);
		}
	}
}
