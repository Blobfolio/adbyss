/*!
# `Adbyss`: Block Lists

TODO: Regex support. (exclude supportxmr, gravatar)
*/

use fyi_msg::{
	MsgKind,
	NiceInt,
};
use rayon::prelude::*;
use regex::Regex;
use std::{
	collections::HashSet,
	ffi::OsStr,
	fmt,
	fs::File,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



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

/// # Flag: Fresh Start.
///
/// This flag excludes existing user host entries (instead of merging them with
/// the shitlist).
///
/// You almost certainly do not want to enable this when writing to /etc/hosts
/// as it will effectively erase any custom entries you've manually added.
pub const FLAG_FRESH: u8       = 0b0010_0000;

/// # Flag: Summarize
///
/// Print a success message after writing results to a file.
pub const FLAG_SUMMARIZE: u8   = 0b0100_0000;

/// # Flag: Non-Interactive Mode.
///
/// This flag bypasses the confirmation when writing to an existing file.
pub const FLAG_Y: u8           = 0b1000_0000;

/// # Flag: `AdAway` Data URL.
const SRC_ADAWAY: &str = "https://adaway.org/hosts.txt";

/// # Flag: `StevenBlack` Data URL.
const SRC_STEVENBLACK: &str = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";

/// # Flag: `Yoyo` Data URL.
const SRC_YOYO: &str = "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext";



#[derive(Debug)]
/// # Shitlist.
///
/// This struct holds the shitlist data from the specified sources. It follows
/// a builder pattern, so generally should be constructed using the various
/// `with_*` methods, followed by [`build()`](Shitlist::build).
///
/// Results are cumulative, so if you plan on doing this more than once with
/// different setups, instantiate a new oject.
pub struct Shitlist {
	hostfile: PathBuf,
	flags: u8,
	exclude: HashSet<String>,
	regexclude: Vec<Regex>,
	found: HashSet<String>,
	out: Vec<u8>,
}

impl Default for Shitlist {
	fn default() -> Self {
		Self {
			hostfile: PathBuf::from("/etc/hosts"),
			flags: 0,
			exclude: HashSet::new(),
			regexclude: Vec::new(),
			found: HashSet::with_capacity(60000),
			out: Vec::with_capacity(60000),
		}
	}
}

impl fmt::Display for Shitlist {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl Shitlist {
	#[must_use]
	/// # With Flags.
	///
	/// Enable one or more flags. See the module documentation for details.
	pub const fn with_flags(mut self, flags: u8) -> Self {
		self.flags |= flags;
		self
	}

	#[must_use]
	/// # With Hostfile.
	///
	/// This method allows you to use a hostfile other than `/etc/hosts`.
	///
	/// This path may be used both for input and output. Unless [`FLAG_FRESH`]
	/// is set, custom entries — anything prior to the `# ADBYSS #` block —
	/// will be added to the start of the output. If [`Shitlist::write`] is
	/// called, output will be written to this same path.
	pub fn with_hostfile<P>(mut self, src: P) -> Self
	where P: AsRef<Path> {
		self.set_hostfile(src);
		self
	}

	#[must_use]
	/// # With Source.
	///
	/// Enable a source using a string slice instead of the corresponding flag.
	pub fn with_source<S>(mut self, src: S) -> Self
	where S: AsRef<str> {
		self.set_source(src);
		self
	}

	#[must_use]
	/// # With Sources.
	///
	/// Enable one or more sources from strings instead of setting the
	/// corresponding flags.
	pub fn with_sources<I>(mut self, src: I) -> Self
	where I: IntoIterator<Item=String> {
		self.set_sources(src);
		self
	}

	#[must_use]
	/// # With Manual Entries.
	///
	/// Add one or more arbitrary domains to the shitlist. This is primarily
	/// intended for cases where you want to blackhole hosts the authoritative
	/// sources don't know about.
	pub fn with<I>(mut self, extras: I) -> Self
	where I: IntoIterator<Item=String> {
		self.include(extras);
		self
	}

	#[must_use]
	/// # Exclude Entries.
	///
	/// Exclude one or more arbitrary domains from the shitlist. This is
	/// primarily intended for cases where an authoritative source blackholes
	/// an address you want to be able to visit, e.g. `supportxmr.com`.
	pub fn without<I>(mut self, excludes: I) -> Self
	where I: IntoIterator<Item=String> {
		self.exclude(excludes);
		self
	}

	#[must_use]
	/// # Exclude Entries.
	///
	/// This is the same as [`Shitlist::without`] except that it takes string
	/// slices that form regular expressions.
	///
	/// Note, all domains are normalized to lowercase, so your expressions can
	/// focus on that without having to use an `(?i)` flag.
	pub fn without_regex<I>(mut self, excludes: I) -> Self
	where I: IntoIterator<Item=String> {
		self.regexclude(excludes);
		self
	}

	/// # Set Flags.
	///
	/// Enable one or more flags. See the module documentation for details.
	pub fn set_flags(&mut self, flags: u8) {
		self.flags |= flags;
	}

	/// # With Hostfile.
	///
	/// This method allows you to use a hostfile other than `/etc/hosts`.
	///
	/// This path may be used both for input and output. Unless [`FLAG_FRESH`]
	/// is set, custom entries — anything prior to the `# ADBYSS #` block —
	/// will be added to the start of the output. If [`Shitlist::write`] is
	/// called, output will be written to this same path.
	pub fn set_hostfile<P>(&mut self, src: P)
	where P: AsRef<Path> {
		if let Ok(src) = std::fs::canonicalize(src) {
			self.hostfile = src;
		}
	}

	/// # Set Source.
	///
	/// Enable a source using a string slice instead of the corresponding flag.
	pub fn set_source<S>(&mut self, src: S)
	where S: AsRef<str> {
		match src.as_ref().trim().to_lowercase().as_str() {
			"adaway" => { self.flags |= FLAG_ADAWAY; },
			"adbyss" => { self.flags |= FLAG_ADBYSS; },
			"stevenblack" => { self.flags |= FLAG_STEVENBLACK; },
			"yoyo" => { self.flags |= FLAG_YOYO; },
			_ => {},
		}
	}

	/// # Set Sources.
	///
	/// Enable one or more sources from strings instead of setting the
	/// corresponding flags.
	pub fn set_sources<I>(&mut self, src: I)
	where I: IntoIterator<Item=String> {
		src.into_iter().for_each(|x| self.set_source(x));
	}

	/// # Set Manual Entries.
	///
	/// Add one or more arbitrary domains to the shitlist. This is primarily
	/// intended for cases where you want to blackhole hosts the authoritative
	/// sources don't know about.
	pub fn include<I>(&mut self, extras: I)
	where I: IntoIterator<Item=String> {
		self.found.extend(
			extras.into_iter()
				.filter_map(|x| crate::sanitize_domain(&x))
		);
		self.strip_excludes();
		self.build_out();
	}

	/// # Exclude Entries.
	///
	/// Exclude one or more arbitrary domains from the shitlist. This is
	/// primarily intended for cases where an authoritative source blackholes
	/// an address you want to be able to visit, e.g. `supportxmr.com`.
	pub fn exclude<I>(&mut self, excludes: I)
	where I: IntoIterator<Item=String> {
		self.exclude.extend(excludes);
	}

	/// # Exclude Entries (Regular Expression).
	///
	/// This is the same as [`Shitlist::exclude`] except it takes regular
	/// expressions.
	pub fn regexclude<I>(&mut self, excludes: I)
	where I: IntoIterator<Item=String> {
		// Add them if we can.
		excludes.into_iter()
			.filter_map(|x| Regex::new(&x).ok())
			.for_each(|x| {
				self.regexclude.push(x);
			});
	}

	/// # Build.
	///
	/// This method can be called after all of the settings have been set to
	/// fetch and parse the shitlist results from the selected sources. The
	/// number of new records added is returned.
	///
	/// This method does not output anything. See [`Shitlist::as_str`],
	/// [`Shitlist::write`], and [`Shitlist::write_to`] to actually *do*
	/// something with the results.
	pub fn build(&mut self) -> usize {
		let before: usize = self.found.len();

		// Find the sources and whatnot.
		self.found.par_extend(
			[
				FLAG_ADAWAY,
				FLAG_ADBYSS,
				FLAG_STEVENBLACK,
				FLAG_YOYO,
			].par_iter()
				.filter(|x| 0 != self.flags & *x)
				.flat_map(|x|
					match *x {
						FLAG_ADAWAY => parse_adaway_hosts(&fetch_url(SRC_ADAWAY)),
						FLAG_ADBYSS => parse_list(
							include_str!("../skel/adbyss.shitlist")
						),
						FLAG_STEVENBLACK => parse_etc_hosts(&fetch_url(SRC_STEVENBLACK)),
						FLAG_YOYO => parse_adaway_hosts(&fetch_url(SRC_YOYO)),
						_ => unreachable!(),
					}
				)
				.collect::<HashSet<String>>()
		);

		// Post-processing.
		self.strip_excludes();
		self.build_out();

		// We're done!
		self.found.len() - before
	}

	#[must_use]
	/// # As Str.
	///
	/// Return the output as a string slice.
	pub fn as_str(&self) -> &str {
		unsafe { std::str::from_utf8_unchecked(&self.out) }
	}

	#[must_use]
	/// # Is Empty.
	///
	/// Returns `true` if no shitlisted hosts have been found.
	pub fn is_empty(&self) -> bool { self.found.is_empty() }

	#[must_use]
	/// # Length.
	///
	/// Return the number of entries found.
	pub fn len(&self) -> usize { self.found.len() }

	/// # Write Changes to Hostfile.
	///
	/// Write the changes to the input hostfile. This method first tries an
	/// atomic write strategy, but will fall back to a clobbery one if that
	/// fails. (Often times `/etc/hosts` is mounted in such a way that it
	/// cannot be "renamed", as is done with atomic writes.)
	///
	/// Use the [`FLAG_BACKUP`] flag to backup the original entry before
	/// committing the changes.
	///
	/// This method will print an error and exit with a status code of `1` if
	/// it is unable to read from or write to the relevant path(s).
	pub fn write(&self) {
		self.write_to(&self.hostfile);
	}

	#[allow(trivial_casts)] // Triviality is required!
	/// # Write Changes to File.
	///
	/// Write the changes to an arbitrary file. This method first tries an
	/// atomic write strategy, but will fall back to a clobbery one if that
	/// fails. (Often times `/etc/hosts` is mounted in such a way that it
	/// cannot be "renamed", as is done with atomic writes.)
	///
	/// Use the [`FLAG_BACKUP`] flag to backup the original entry before
	/// committing the changes.
	///
	/// If the destination already exists, you will be prompted before any
	/// changes are written, giving you a chance to abort, unless the [`FLAG_Y`]
	/// flag has been set.
	///
	/// This method will print an error and exit with a status code of `1` if
	/// it is unable to read from or write to the relevant path(s).
	pub fn write_to<P>(&self, dst: P)
	where P: AsRef<Path> {
		let mut dst: PathBuf = dst.as_ref().to_path_buf();

		// Does it already exist?
		if dst.exists() {
			dst = dst.canonicalize().unwrap();

			// Can't be a directory.
			if dst.is_dir() {
				MsgKind::Error
					.into_msg(&format!("Invalid hostfile: {:?}", dst))
					.eprintln();
				std::process::exit(1);
			}

			// Prompt about writing it?
			if
				0 == self.flags & FLAG_Y &&
				! MsgKind::Confirm
					.into_msg(&format!("Write {} hosts to {:?}?", NiceInt::from(self.len()).as_str(), dst))
					.prompt()
			{
				MsgKind::Warning.into_msg("Operation aborted.").eprintln();
				return;
			}

			// Back it up!
			if 0 != self.flags & FLAG_BACKUP {
				let dst2: PathBuf = PathBuf::from(OsStr::from_bytes(&[
					unsafe { &*(dst.as_os_str() as *const OsStr as *const [u8]) },
					b".adbyss.bak"
				].concat()));

				// Copy the original.
				if let Ok(txt) = std::fs::read_to_string(&dst) {
					// Try to write atomically, fall back to clobbering, or
					// report error.
					if write_to_file(&dst2, txt.as_bytes()).is_err() && write_nonatomic_to_file(&dst2, txt.as_bytes()).is_err() {
						MsgKind::Error
							.into_msg(&format!("Unable to write backup {:?}; root privileges may be required.", dst2))
							.eprintln();
						std::process::exit(1);
					}
				}
				else {
					MsgKind::Error
						.into_msg(&format!("Unable to read {:?}; root privileges may be required.", dst2))
						.eprintln();
					std::process::exit(1);
				}

				// Explain what we've done.
				if 0 != self.flags & FLAG_SUMMARIZE {
					MsgKind::Notice
						.into_msg(&format!(
							"The original hostfile has been backed up to {:?}.",
							dst2
						))
						.println();
				}
			}

			// Try to write atomically, fall back to clobbering, or report error.
			if write_to_file(&dst, &self.out).is_err() && write_nonatomic_to_file(&dst, &self.out).is_err() {
				MsgKind::Error
					.into_msg(&format!("Unable to write to hostfile {:?}; root privileges may be required.", dst))
					.eprintln();
				std::process::exit(1);
			}

			// Summarize?
			if 0 != self.flags & FLAG_SUMMARIZE {
				MsgKind::Success
					.into_msg(&format!(
						"Wrote {} blackholed hosts to {:?}.",
						NiceInt::from(self.len()).as_str(),
						dst
					))
					.println();
			}
		}
	}

	/// # Compile Output.
	///
	/// This compiles a new hosts file using the data found. Unless [`FLAG_FRESH`]
	/// is set, this will include the non-adbyss contents of the original
	/// hostfile, followed by the shitlist.
	///
	/// If the original hostfile cannot be read, the program will print an error
	/// and exit with a status code of `1`. This does not apply in cases where
	/// [`FLAG_FRESH`] is set.
	fn build_out(&mut self) {
		self.out.clear();

		// Load existing hosts.
		if 0 == self.flags & FLAG_FRESH {
			if let Ok(mut txt) = std::fs::read_to_string(&self.hostfile) {
				// If the watermark already exists, remove it and all following.
				if let Some(idx) = txt.find(crate::WATERMARK) {
					txt.truncate(idx);
				}

				self.out.extend_from_slice(txt.trim().as_bytes());
				self.out.push(b'\n');
				self.out.push(b'\n');
			}
			else {
				MsgKind::Error
					.into_msg(&format!("Unable to read {:?}; root privileges may be required.", self.hostfile))
					.eprintln();

				std::process::exit(1);
			}
		}

		// Add marker.
		self.out.extend_from_slice(crate::WATERMARK.as_bytes());
		self.out.push(b'\n');

		// Add all of our results!
		let mut found: Vec<String> = self.found.iter().cloned().collect();
		found.par_sort();

		found.iter().for_each(|x| {
			self.out.extend_from_slice(b"\n0.0.0.0 ");
			self.out.extend_from_slice(x.as_bytes());
		});

		self.out.push(b'\n');
	}

	/// # Strip Ignores.
	///
	/// This removes any excluded domains from the results.
	fn strip_excludes(&mut self) {
		if
			! self.found.is_empty() &&
			(! self.exclude.is_empty() || ! self.regexclude.is_empty())
		{
			self.found.par_iter()
				.filter(
					|x|
					self.exclude.contains(x.as_str()) ||
					self.regexclude.iter().any(|r| r.is_match(x))
				)
				.cloned()
				.collect::<HashSet<String>>()
					.iter().for_each(|x| {
						self.found.remove(x);
					});
		}
	}
}

/// # Fetch URL.
///
/// This is just a GET wrapper that returns the response as a string.
fn fetch_url(url: &str) -> String {
	ureq::get(url)
		.call()
		.into_string()
		.unwrap_or_default()
}

/// # Parse Hosts Format.
///
/// Most data sources format results in something akin to the final `/etc/hosts`
/// format, where each line looks like `0.0.0.0 somehost.com`.
///
/// This extracts the hosts from such lines, ignoring comments and the like, as
/// well as entries with other IPs assigned to them.
fn parse_etc_hosts(raw: &str) -> HashSet<String> {
	lazy_static::lazy_static! {
		static ref RE: Regex = Regex::new(r"((^0\.0\.0\.0\s+)|(#.*$))").unwrap();
	}

	raw.lines()
		.filter_map(|x|
			if x.trim().starts_with("0.0.0.0 ") {
				Some(RE.replace_all(x, "").split_whitespace().map(String::from).collect::<Vec<String>>())
			}
			else { None }
		)
		.flatten()
		.filter_map(|x| crate::sanitize_domain(x.as_str()))
		.collect()
}

/// # Parse `AdAway` Hosts Format.
///
/// The `AdAway` sources send targets to `127.0.0.1` instead of `0.0.0.0`; this
/// just quickly patches such data so that it can then be parsed using
/// [`parse_etc_hosts`].
fn parse_adaway_hosts(raw: &str) -> HashSet<String> {
	lazy_static::lazy_static! {
		static ref RE: Regex = Regex::new(r"(?m)^127\.0\.0\.1[\t ]").unwrap();
	}
	parse_etc_hosts(&RE.replace_all(raw, "0.0.0.0 "))
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

/// # Atomic Write Helper.
///
/// This method writes data to a temporary file, then replaces the target with
/// it. This is safer than writing data directly to the target as it (mostly)
/// moots the risk of panic-related partial writes.
fn write_to_file(path: &PathBuf, data: &[u8]) -> Result<(), ()> {
	use std::io::Write;

	let mut file = tempfile_fast::Sponge::new_for(path).map_err(|_| ())?;
	file.write_all(data).map_err(|_| ())?;
	file.commit().map_err(|_| ())?;

	Ok(())
}

/// # Write Helper.
///
/// This is a fallback writer that writes data directly to the destination.
///
/// It is often needed for special system files like `/etc/hosts` that may not
/// allow atomic-style rename-replacing.
fn write_nonatomic_to_file(path: &PathBuf, data: &[u8]) -> Result<(), ()> {
	use std::io::Write;

	let mut file = File::create(path).map_err(|_| ())?;
	file.write_all(data).map_err(|_| ())?;
	file.flush().map_err(|_| ())?;

	Ok(())
}



#[cfg(test)]
mod tests {
	use super::*;

	const STUB: &str = r"# AdAway default blocklist
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
0.0.0.0 crash.163.com # Comment.com here!
0.0.0.0 crashlytics.163.com
0.0.0.0 iad.g.163.com

# [1mobile.com]
0.0.0.0 ads.1mobile.com
0.0.0.0 api.1mobile.com";

	#[test]
	fn t_parse_host_fmt() {
		let mut test: Vec<String> = parse_etc_hosts(STUB).into_iter().collect();
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
