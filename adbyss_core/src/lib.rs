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
	FLAG_COMPACT,
	FLAG_Y,
};



lazy_static::lazy_static! {
	// Load the Public Suffix List only once.
	static ref PSL: publicsuffix::List = publicsuffix::List::from_str(
		include_str!("../skel/public_suffix_list.dat")
	).expect("Unable to load Public Suffix list.");
}

#[must_use]
/// # Sanitize Domain.
///
/// This ensures the domain is correctly formatted and has a recognized TLD.
pub fn sanitize_domain(dom: &str) -> Option<String> {
	use publicsuffix::Host;

	// Look for the domain any which way it happens to be.
	if let Ok(Host::Domain(dom)) = PSL.parse_str(dom.trim()) {
		// It should have a root and a suffix.
		if
			dom.is_icann() &&
			dom.suffix().is_some() &&
			dom.has_known_suffix() &&
			dom.root().is_some()
		{
			return idna::domain_to_ascii_strict(dom.full())
				.ok()
				.filter(|x| ! x.is_empty());
		}
	}

	None
}

#[must_use]
/// # Domain Suffix.
///
/// This extracts the domain's suffix, if any.
pub(crate) fn domain_suffix(dom: &str) -> Option<String> {
	use publicsuffix::Host;

	// Look for the domain any which way it happens to be.
	if let Ok(Host::Domain(dom)) = PSL.parse_str(dom.trim()) {
		if dom.has_known_suffix() {
			dom.suffix().filter(|x| x.is_ascii()).map(String::from)
		}
		else { None }
	}
	else { None }
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
			("☺.com", Some(String::from("xn--74h.com"))),
			("www.☺.com", Some(String::from("www.xn--74h.com"))),
			("www.xn--74h.com", Some(String::from("www.xn--74h.com"))),
		].iter() {
			assert_eq!(sanitize_domain(a), *b);
		}
	}
}
