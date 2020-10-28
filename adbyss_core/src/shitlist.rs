/*!
# `Adbyss`: Block Lists
*/

use std::collections::HashSet;



#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
/// # Shitlist.
pub enum Shitlist {
	/// None.
	None,
	/// AdAway.
	AdAway,
	/// Adbyss.
	Adbyss,
	/// Namely cryptojackers.
	Marfjeh,
	/// Tracking, malware, ads, etc.
	StevenBlack,
	/// Tracking, malware, ads, etc.
	Yoyo,
}

impl Default for Shitlist {
	fn default() -> Self {
		Self::None
	}
}

impl From<&str> for Shitlist {
	fn from(src: &str) -> Self {
		match src.to_lowercase().as_str() {
			"adaway" => Self::AdAway,
			"adbyss" => Self::Adbyss,
			"marfjeh" => Self::Marfjeh,
			"stevenblack" => Self::StevenBlack,
			"yoyo" => Self::Yoyo,
			_ => Self::None,
		}
	}
}

impl Shitlist {
	#[must_use]
	/// # As Str.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::None => "",
			Self::AdAway => "AdAway",
			Self::Adbyss => "Adbyss",
			Self::Marfjeh => "Marfjeh",
			Self::StevenBlack => "StevenBlack",
			Self::Yoyo => "Yoyo",
		}
	}

	#[must_use]
	/// # Fetch.
	pub fn fetch(self) -> HashSet<String> {
		match self {
			Self::None => HashSet::new(),
			Self::AdAway | Self::Yoyo =>
				if let Ok(raw) = fetch_url(self.url()) {
					// AdAway records the IP as localhost instead of the zero
					// hole. We can convert that real quick, then use our host
					parse_etc_hosts(&raw.replace("127.0.0.1", "0.0.0.0"))
				}
				else { HashSet::new() },
			Self::Adbyss => parse_list(include_str!("../skel/adbyss.shitlist")),
			Self::Marfjeh =>
				if let Ok(raw) = fetch_url(self.url()) {
					parse_list(&raw)
				}
				else { HashSet::new() },
			Self::StevenBlack =>
				if let Ok(raw) = fetch_url(self.url()) {
					// AdAway records the IP as localhost instead of the zero
					// hole. We can convert that real quick, then use our host
					parse_etc_hosts(&raw)
				}
				else { HashSet::new() },
		}
	}

	#[must_use]
	/// # Data URL.
	pub const fn url(self) -> &'static str {
		match self {
			Self::None | Self::Adbyss => "",
			Self::AdAway => "https://adaway.org/hosts.txt",
			Self::Marfjeh => "https://raw.githubusercontent.com/Marfjeh/coinhive-block/master/domains",
			Self::StevenBlack => "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
			Self::Yoyo => "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
		}
	}
}

/// # Fetch URL.
fn fetch_url(url: &str) -> Result<String, String> {
	ureq::get(url)
		.call()
		.into_string()
		.map_err(|e| e.to_string())
}

/// # Parse Host Format.
///
/// The lines to include look like an `/etc/hosts` file, each line beginning
/// with `0.0.0.0` after which the affected host(s) appear, whitespace-
/// separated.
///
/// This works for several of our lists.
fn parse_etc_hosts(raw: &str) -> HashSet<String> {
	raw.lines()
		.filter_map(|x|
			if x.trim().starts_with("0.0.0.0 ") {
				Some(x.split_whitespace().skip(1))
			}
			else { None }
		)
		.flatten()
		.filter_map(crate::sanitize_domain)
		.collect()
}

/// # Parse List.
///
/// This is essentially just a big ol' list of domains.
fn parse_list(raw: &str) -> HashSet<String> {
	raw.lines()
		.filter_map(|x|
			if ! x.is_empty() && ! x.starts_with('#') {
				crate::sanitize_domain(x)
			}
			else { None }
		)
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_parse_host_fmt() {
		let mut test: Vec<String> = parse_etc_hosts(r"# AdAway default blocklist
# Blocking mobile ad providers and some analytics providers
#
# Project home page:
# https://github.com/AdAway/adaway.github.io/
#
# Fetch the latest version of this file:
# https://raw.githubusercontent.com/AdAway/adaway.github.io/master/hosts.txt
#
# License:
# CC Attribution 3.0 (http://creativecommons.org/licenses/by/3.0/)
#
# Contributions by:
# Kicelo, Dominik Schuermann.
# Further changes and contributors maintained in the commit history at
# https://github.com/AdAway/adaway.github.io/commits/master
#
# Contribute:
# Create an issue at https://github.com/AdAway/adaway.github.io/issues
#

0.0.0.0  localhost
::1  localhost

# [163.com]
0.0.0.0 analytics.163.com
0.0.0.0 crash.163.com
0.0.0.0 crashlytics.163.com
0.0.0.0 iad.g.163.com

# [1mobile.com]
0.0.0.0 ads.1mobile.com
0.0.0.0 api.1mobile.com").into_iter().collect();
		test.sort();

		assert_eq!(
			test,
			vec![
				String::from("ads.1mobile.com"),
				String::from("analytics.163.com"),
				String::from("api.1mobile.com"),
				String::from("crash.163.com"),
				String::from("crashlytics.163.com"),
				String::from("iad.g.163.com"),
			]
		);
	}
}
