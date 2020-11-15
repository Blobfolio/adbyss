/*!
# `Adbyss`: Block Lists
*/

use adbyss_psl::Domain;
use fyi_msg::{
	MsgKind,
	NiceInt,
};
use rayon::prelude::*;
use regex::Regex;
use std::{
	collections::{
		HashMap,
		HashSet,
	},
	ffi::OsStr,
	fmt,
	fs::File,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};
use strum::{
	IntoEnumIterator,
	EnumIter,
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
const MAX_LINE: usize = 245;



#[derive(Clone, Copy, PartialEq)]
/// Watermark.
///
/// This is used to match the boundary between the custom hostfile entries and
/// Adbyss' contributions.
enum Watermark {
	Zero,
	One,
	Two,
	Three,
}

impl Watermark {
	/// The Next Entry.
	const fn next(self) -> Self {
		match self {
			Self::Zero => Self::One,
			Self::One => Self::Two,
			Self::Two | Self::Three => Self::Three,
		}
	}

	/// The Line.
	const fn as_str(self) -> &'static str {
		match self {
			Self::Zero => "",
			Self::One | Self::Three => "##########",
			Self::Two => "# ADBYSS #",
		}
	}

	/// Match the Watermark.
	///
	/// If it matches the next expected text, the next step is returned,
	/// otherwise it resets to zero.
	fn is_match(self, line: &str) -> Self {
		let next = self.next();
		if line == next.as_str() { next }
		else { Self::Zero }
	}
}



#[derive(Debug, Copy, Clone, EnumIter, Hash, Eq, PartialEq)]
enum ShitlistSource {
	/// AdAway.
	AdAway,
	/// Adbyss.
	Adbyss,
	/// StevenBlack.
	StevenBlack,
	/// Yoyo.
	Yoyo,
}

impl ShitlistSource {
	/// # As Byte (Flag).
	///
	/// Return the equivalent flag for the source.
	const fn as_byte(self) -> u8 {
		match self {
			Self::AdAway => FLAG_ADAWAY,
			Self::Adbyss => FLAG_ADBYSS,
			Self::StevenBlack => FLAG_STEVENBLACK,
			Self::Yoyo => FLAG_YOYO,
		}
	}

	/// # Fetch by Flag.
	///
	/// This fetches and returns a single host collection given the flags.
	fn fetch(flags: u8) -> Result<HashSet<String>, String> {
		let mut out: HashSet<String> = HashSet::new();

		for x in Self::iter().filter(|x| 0 != flags & x.as_byte()) {
			match x.parse() {
				Ok(y) => {
					out.par_extend(y);
				},
				Err(e) => return Err(e),
			}
		}

		Ok(out)
	}

	/// # Parse Raw.
	///
	/// Fetch and parse the raw source data.
	fn parse(self) -> Result<HashSet<String>, String> {
		Ok(match self {
			Self::AdAway | Self::Yoyo => parse_adaway_hosts(&self.raw()?),
			Self::Adbyss => {
				let mut hs: HashSet<String> = HashSet::with_capacity(20);

				hs.insert(String::from("api.triptease.io"));
				hs.insert(String::from("collect.snitcher.com"));
				hs.insert(String::from("ct.pinterest.com"));
				hs.insert(String::from("guest-experience.triptease.io"));
				hs.insert(String::from("js.trendmd.com"));
				hs.insert(String::from("medtargetsystem.com"));
				hs.insert(String::from("onboard.triptease.io"));
				hs.insert(String::from("rum-static.pingdom.net"));
				hs.insert(String::from("s.pinimg.com"));
				hs.insert(String::from("shareasale-analytics.com"));
				hs.insert(String::from("snid.snitcher.com"));
				hs.insert(String::from("snitcher.com"));
				hs.insert(String::from("static-meta.triptease.io"));
				hs.insert(String::from("static.triptease.io"));
				hs.insert(String::from("trendmd.com"));
				hs.insert(String::from("triptease.io"));
				hs.insert(String::from("www.medtargetsystem.com"));
				hs.insert(String::from("www.snitcher.com"));
				hs.insert(String::from("www.trendmd.com"));
				hs.insert(String::from("www.triptease.io"));

				hs
			},
			Self::StevenBlack => parse_blackhole_hosts(&self.raw()?),
		})
	}

	/// # Fetch Raw.
	///
	/// This fetches and returns the raw, remote data for a given source.
	fn raw(self) -> Result<String, String> {
		match self {
			Self::AdAway |
			Self::StevenBlack |
			Self::Yoyo => fetch_url(self.url()),
			Self::Adbyss => Ok(String::new()),
		}
	}

	/// # Source URL.
	///
	/// For remote hosts, return the URL where data is found.
	const fn url(self) -> &'static str {
		match self {
			Self::AdAway => "https://adaway.org/hosts.txt",
			Self::Adbyss => "",
			Self::StevenBlack => "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
			Self::Yoyo => "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
		}
	}
}



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
	/// This path may be used both for input and output. When writing, anything
	/// prior to the `# ADBYSS #` block will be retained (i.e. only the Adbyss
	/// bits will be modified).
	pub fn with_hostfile<P>(mut self, src: P) -> Self
	where P: AsRef<Path> {
		self.set_hostfile(src);
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

	/// # Disable Flags.
	///
	/// Disable one or more flags. See the module documentation for details.
	pub fn disable_flags(&mut self, flags: u8) {
		self.flags &= ! flags;
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
	/// This path may be used both for input and output. When writing, anything
	/// prior to the `# ADBYSS #` block will be retained (i.e. only the Adbyss
	/// bits will be modified).
	pub fn set_hostfile<P>(&mut self, src: P)
	where P: AsRef<Path> {
		if let Ok(src) = std::fs::canonicalize(src) {
			self.hostfile = src;
		}
	}

	/// # Set Manual Entries.
	///
	/// Add one or more arbitrary domains to the shitlist. This is primarily
	/// intended for cases where you want to blackhole hosts the authoritative
	/// sources don't know about.
	pub fn include<I>(&mut self, extras: I)
	where I: IntoIterator<Item=String> {
		self.found.extend(extras.into_iter().filter_map(crate::sanitize_domain));
		let _ = self.build_out();
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
	pub fn build(mut self) -> Result<Self, String> {
		self.found.par_extend(ShitlistSource::fetch(self.flags)?);

		// Post-processing.
		self.build_out()?;

		// We're done!
		Ok(self)
	}

	#[must_use]
	/// # As Str.
	///
	/// Return the output as a string slice.
	pub fn as_str(&self) -> &str {
		unsafe { std::str::from_utf8_unchecked(&self.out) }
	}

	#[must_use]
	/// # As Str.
	///
	/// Return the output as a string slice.
	pub fn as_bytes(&self) -> &[u8] { &self.out }

	#[must_use]
	/// # Take Found
	///
	/// Consume the struct and return a sorted vector of the qualifying
	/// blackholeable hosts.
	pub fn into_vec(mut self) -> Vec<String> {
		let mut found: Vec<String> = self.found.par_drain()
			.filter(|x| x.len() <= MAX_LINE)
			.collect();
		found.par_sort();
		found
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

	/// # Stub.
	///
	/// Return the user portion of the specified hostfile.
	pub fn hostfile_stub(&self) -> Result<String, String> {
		use std::io::{
			BufRead,
			BufReader,
		};

		// Load existing hosts.
		let mut txt: String = String::with_capacity(512);
		let mut watermark: Watermark = Watermark::Zero;

		for line in File::open(&self.hostfile)
			.map(BufReader::new)
			.map_err(|_| format!("Unable to read hostfile: {:?}", self.hostfile))?
			.lines()
			.filter_map(std::result::Result::ok)
		{
			// We'll want to stop once we have absorbed the watermark.
			watermark = watermark.is_match(&line);
			if Watermark::Three == watermark {
				// Erase the two lines we've already written, and trim the
				// end once more for good measure.
				txt.truncate(txt[..txt.len()-23].trim_end().len());
				txt.push('\n');
				break;
			}

			txt.push_str(line.trim());
			txt.push('\n');
		}

		Ok(txt)
	}

	/// # Uninstall Adbyss Rules
	///
	/// This will remove all of Adbyss' blackhole entries from the given
	/// hostfile.
	pub fn unwrite(&self) -> Result<(), String> {
		// Prompt about writing it?
		if
			0 == self.flags & FLAG_Y &&
			! MsgKind::Confirm
				.into_msg(&format!(
					"Remove all Adbyss blackhole entries from {:?}?",
					&self.hostfile
				))
				.prompt()
		{
			return Err(String::from("Operation aborted."));
		}

		// Try to write atomically, fall back to clobbering, or report error.
		self.hostfile_stub()
			.and_then(|stub| write_to_file(&self.hostfile, stub.as_bytes()))
	}

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
	pub fn write(&self) -> Result<(), String> {
		self.write_to(&self.hostfile)
	}

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
	pub fn write_to<P>(&self, dst: P) -> Result<(), String>
	where P: AsRef<Path> {
		let mut dst: PathBuf = dst.as_ref().to_path_buf();

		// Does it already exist?
		if dst.exists() {
			dst = dst.canonicalize()
				.ok()
				.filter(|x| ! x.is_dir())
				.ok_or_else(|| String::from("Hostfile cannot be a directory."))?;

			// Prompt about writing it?
			if
				0 == self.flags & FLAG_Y &&
				! MsgKind::Confirm
					.into_msg(&format!(
						"Write {} hosts to {:?}?",
						NiceInt::from(self.len()).as_str(),
						dst
					))
					.prompt()
			{
				return Err(String::from("Operation aborted."));
			}

			self.backup(&dst)?;
		}

		// Try to write atomically, fall back to clobbering, or report error.
		write_to_file(&dst, &self.out)
	}

	/// # Add www.domain.com TLDs.
	///
	/// This assumes that if something with a `www.` prefix is being
	/// blacklisted, it should also be blacklisted without the `www.`.
	///
	/// Note: The reverse is not enforced as that would be madness!
	fn add_www_tlds(&mut self) {
		if self.found.is_empty() { return; }

		let extra: HashSet<String> = self.found
			.par_iter()
			.filter(|x| x.starts_with("www."))
			.filter_map(|x|
				Domain::parse(x)
					.and_then(|mut x|
						if x.strip_www() { Some(x.take()) }
						else { None }
					)
			)
			.collect();

		if ! extra.is_empty() {
			self.found.par_extend(extra);
		}
	}

	#[allow(trivial_casts)] // Triviality is required!
	/// # Backup.
	fn backup(&self, dst: &PathBuf) -> Result<(), String> {
		// Back it up!
		if 0 != self.flags & FLAG_BACKUP {
			// Tack ".adbyss.bak" onto the original path.
			let dst2: PathBuf = PathBuf::from(OsStr::from_bytes(&[
				unsafe { &*(dst.as_os_str() as *const OsStr as *const [u8]) },
				b".adbyss.bak"
			].concat()));

			// Copy the original, clobbering only as a fallback.
			std::fs::copy(&dst, &dst2)
				.map_err(|_| format!("Unable to backup hostfile: {:?}", dst2))?;
		}

		Ok(())
	}

	/// # Compile Output.
	///
	/// This compiles the output for a new hostfile so that it can be returned
	/// as a slice or written to a file. Only the Adbyss section of the
	/// hostfile — if any — will be modified; any custom host entries appearing
	/// before that block will be retained.
	///
	/// If the original hostfile cannot be read, the program will print an error
	/// and exit with a status code of `1`.
	fn build_out(&mut self) -> Result<(), String> {
		self.out.clear();

		// Pull the stub of the current host, and add any hosts to the
		// exclude list.
		self.out.extend_from_slice({
			let mut txt = self.hostfile_stub()?;
			self.exclude.par_extend(parse_custom_hosts(&txt));
			txt.push('\n');
			txt
		}.as_bytes());

		// Re-clean the found list according to the current excludey bits.
		self.add_www_tlds();
		self.strip_excludes();

		// Add marker.
		self.out.extend_from_slice(format!(
			r#"##########
# ADBYSS #
##########
#
# This section is automatically generated. Any changes you make here will just
# be removed the next time Adbyss is run.
#
# If you have custom host entries to add, place them at the top of this file
# instead. (Anywhere before the start of this section will do.)
#
# Updated: {}
# Blocked: {} garbage hosts
#
# Eat the rich.
#
##########
"#,
			chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z"),
			NiceInt::from(self.found.len()).as_str()
		).as_bytes());

		// Add our results!
		if 0 == self.flags & FLAG_COMPACT { self.found_separate() }
		else { self.found_compact() }
			.iter().for_each(|x| {
				self.out.extend_from_slice(b"\n0.0.0.0 ");
				self.out.extend_from_slice(x.as_bytes());
			});

		self.out.push(b'\n');

		Ok(())
	}

	#[allow(clippy::comparison_chain)] // We're only matching two branches.
	#[allow(clippy::filter_map)] // This is confusing.
	/// # Found: Compact.
	///
	/// This merges TLDs and their subdomains together to reduce the number of
	/// lines (and overall byte size), but without going overboard.
	fn found_compact(&self) -> Vec<String> {
		// Start by building up a map keyed by root domain...
		let mut found: Vec<String> = self.found
			.iter()
			.filter_map(Domain::parse)
			.fold(
				HashMap::<u64, Vec<String>>::with_capacity(self.found.len()),
				|mut acc, dom| {
					let hash: u64 = fyi_msg::utility::hash64(dom.tld().as_bytes());

					acc.entry(hash)
						.or_insert_with(Vec::new)
						.push(dom.take());

					acc
				}
			)
			// Now run through each set to build out the lines.
			.par_drain()
			.flat_map(|(_k, mut x)| {
				// We have to split this into multiple lines so it can
				// fit.
				let mut out: Vec<String> = Vec::new();
				let mut line: String = String::new();

				// Split on whitespace.
				x.sort();
				x.iter().for_each(|x| {
					if line.len() + 1 + x.len() <= MAX_LINE {
						if ! line.is_empty() {
							line.push(' ');
						}
						line.push_str(x);
					}
					else if ! line.is_empty() {
						out.push(line.split_off(0));
						if x.len() <= MAX_LINE {
							line.push_str(x);
						}
					}
				});

				// Add the remainder, if any.
				if ! line.is_empty() {
					out.push(line);
				}

				out
			})
			.collect();
		found.par_sort();
		found
	}

	/// # Found: Straight Sort.
	///
	/// This delivers each host, one per line.
	fn found_separate(&self) -> Vec<String> {
		let mut found: Vec<String> = self.found.par_iter()
			.filter(|x| x.len() <= MAX_LINE)
			.cloned()
			.collect();
		found.par_sort();
		found
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
					.iter()
					.for_each(|x| { self.found.remove(x); });
		}
	}
}

/// # Cache Path From URL.
fn cache_path(url: &str) -> Option<PathBuf> {
	let file: &str = match url {
		"https://adaway.org/hosts.txt" => "_adbyss-adaway.tmp",
		"https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts" => "_adbyss-sb.tmp",
		"https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext" => "_adbyss-yoyo.tmp",
		_ => return None,
	};

	let mut out: PathBuf = std::env::temp_dir();
	if out.is_dir() {
		out.push(file);
		Some(out)
	}
	else { None }
}

/// # Fetch URL.
///
/// This is just a GET wrapper that returns the response as a string.
fn fetch_url(url: &str) -> Result<String, String> {
	match cache_path(url) {
		Some(cache) => {
			// If this raw data was fetched less than an hour ago, simply read
			// and return that instead of asking the Internet for a new copy.
			if cache.is_file() {
				// It takes an epic mapping journey to arrive at the answer
				// without nesting a million if/let statements. Haha.
				if let Some(x) = std::fs::metadata(&cache)
					.and_then(|meta| meta.modified())
					.ok()
					.and_then(|time| time.elapsed().ok())
					.and_then(|secs|
						if 3600 > secs.as_secs() { Some(true) }
						else { None }
					)
					.and_then(|_| std::fs::read_to_string(&cache).ok())
				{
					return Ok(x);
				}
			}

			// Download and cache for next time.
			ureq::get(url).call()
				.into_string()
				.map(|x| {
					let _ = write_to_file(&cache, x.as_bytes());
					x
				})
				.map_err(|e| e.to_string())
		},
		None => ureq::get(url)
			.call()
			.into_string()
			.map_err(|e| e.to_string())
	}
}

/// # Parse Custom Hosts.
///
/// This is used to parse custom hosts out of the user's `/etc/hosts` file.
/// We'll want to exclude these from the blackhole list to prevent duplicates,
/// however unlikely that may be.
fn parse_custom_hosts(raw: &str) -> HashSet<String> {
	lazy_static::lazy_static! {
		// Match comments.
		static ref RE1: Regex = Regex::new(r"#.*$").unwrap();
		// Match IPs. Man, IPv6 is *dramatic*!
		static ref RE2: Regex = Regex::new(r"^(\d+\.\d+\.\d+\.\d+|(([\da-fA-F]{1,4}:){7,7}[\da-fA-F]{1,4}|([\da-fA-F]{1,4}:){1,7}:|([\da-fA-F]{1,4}:){1,6}:[\da-fA-F]{1,4}|([\da-fA-F]{1,4}:){1,5}(:[\da-fA-F]{1,4}){1,2}|([\da-fA-F]{1,4}:){1,4}(:[\da-fA-F]{1,4}){1,3}|([\da-fA-F]{1,4}:){1,3}(:[\da-fA-F]{1,4}){1,4}|([\da-fA-F]{1,4}:){1,2}(:[\da-fA-F]{1,4}){1,5}|[\da-fA-F]{1,4}:((:[\da-fA-F]{1,4}){1,6})|:((:[\da-fA-F]{1,4}){1,7}|:)|fe80:(:[\da-fA-F]{0,4}){0,4}%[\da-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[\d]){0,1}[\d])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[\d]){0,1}[\d])|([\da-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[\d]){0,1}[\d])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[\d]){0,1}[\d])))\s+").unwrap();
	}

	raw.par_lines()
		.filter_map(|x| {
			let line = RE1.replace_all(x.trim(), "");

			if RE2.is_match(&line) {
				Some(
					line.split_whitespace()
						.skip(1)
						.map(String::from)
						.collect::<Vec<String>>()
				).filter(|x| ! x.is_empty())
			}
			else { None }
		})
		.flatten()
		.filter_map(crate::sanitize_domain)
		.collect()
}

/// # Parse Hosts Format.
///
/// Most data sources format results in something akin to the final `/etc/hosts`
/// format, where each line looks like `0.0.0.0 somehost.com`.
///
/// This extracts the hosts from such lines, ignoring comments and the like, as
/// well as entries with other IPs assigned to them.
fn parse_blackhole_hosts(raw: &str) -> HashSet<String> {
	lazy_static::lazy_static! {
		static ref RE: Regex = Regex::new(r"((^0\.0\.0\.0\s+)|(#.*$))").unwrap();
	}

	raw.par_lines()
		.filter_map(|x|
			if x.trim().starts_with("0.0.0.0 ") {
				Some(
					RE.replace_all(x, "")
						.split_whitespace()
						.map(String::from)
						.collect::<Vec<String>>()
				).filter(|x| ! x.is_empty())
			}
			else { None }
		)
		.flatten()
		.filter_map(crate::sanitize_domain)
		.collect()
}

/// # Parse `AdAway` Hosts Format.
///
/// The `AdAway` sources send targets to `127.0.0.1` instead of `0.0.0.0`; this
/// just quickly patches such data so that it can then be parsed using
/// [`parse_blackhole_hosts`].
fn parse_adaway_hosts(raw: &str) -> HashSet<String> {
	lazy_static::lazy_static! {
		static ref RE: Regex = Regex::new(r"(?m)^127\.0\.0\.1[\t ]").unwrap();
	}
	parse_blackhole_hosts(&RE.replace_all(raw, "0.0.0.0 "))
}

/// # Write Helper.
///
/// This method will first attempt an atomic write using `tempfile`, but if
/// that fails — as is common with `/etc/hosts` — it will try a nonatomic write
/// instead.
fn write_to_file(path: &PathBuf, data: &[u8]) -> Result<(), String> {
	use std::io::Write;

	// Try an atomic write first.
	tempfile_fast::Sponge::new_for(path)
		.and_then(|mut file| file.write_all(data).and_then(|_| file.commit()))
		.or_else(|_| File::create(path)
			.and_then(|mut file| file.write_all(data).and_then(|_| file.flush()))
		)
		.map_err(|_| format!("Unable to write to hostfile: {:?}", path))
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
	fn t_parse_blackhole_hosts() {
		let mut test: Vec<String> = parse_blackhole_hosts(STUB).into_iter().collect();
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

	#[test]
	fn t_parse_custom_hosts() {
		let mut test: Vec<String> = parse_custom_hosts(r#"#############
# Localhost #
#############

127.0.0.1 localhost
127.0.1.1 Computer
127.0.0.1 my-dev.loc some-other.loc
172.19.0.2 docker-mysql

##################
# Manual Records #
##################

140.82.113.4 github.com www.github.com
100.100.100.1 0.nextyourcontent.com
2600:3c00::f03c:91ff:feae:ff2 blobfolio.com

########
# IPv6 #
########

::1     ip6-localhost ip6-loopback domain.com www.domain.com
fe00::0 ip6-localnet
ff00::0 ip6-mcastprefix
ff02::1 ip6-allnodes
ff02::2 ip6-allrouters"#).into_iter().collect();
		test.sort();

		assert_eq!(
			test,
			vec![
				String::from("0.nextyourcontent.com"),
				String::from("blobfolio.com"),
				String::from("domain.com"),
				String::from("github.com"),
				String::from("www.domain.com"),
				String::from("www.github.com"),
			]
		);
	}
}
