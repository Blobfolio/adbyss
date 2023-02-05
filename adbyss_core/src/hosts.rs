/*!
# `Adbyss`: Hosts
*/

use adbyss_psl::Domain;
use crate::{
	AdbyssError,
	FLAG_BACKUP,
	FLAG_COMPACT,
	FLAG_Y,
	MAX_LINE,
	Source,
};
use fyi_msg::confirm;
use dactyl::{
	NiceU64,
	NoHash,
};
use rayon::{
	iter::{
		IntoParallelIterator,
		IntoParallelRefIterator,
		ParallelExtend,
		ParallelIterator,
	},
	prelude::{
		ParallelSliceMut,
		ParallelString,
	},
};
use regex::RegexSet;
use std::{
	cmp::Ordering,
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
use utc2k::FmtUtc2k;



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



#[derive(Debug)]
/// # Shitlist.
///
/// This struct holds the shitlist data from the specified sources. It follows
/// a builder pattern, so generally should be constructed using the various
/// `with_*` methods, followed by [`build()`](Shitlist::build).
///
/// Results are cumulative, so if you plan on doing this more than once with
/// different setups, instantiate a new object.
pub struct Shitlist {
	hostfile: PathBuf,
	flags: u8,
	exclude: HashSet<Domain>,
	regexclude: Option<RegexSet>,
	found: HashSet<Domain>,
	out: Vec<u8>,
}

impl Default for Shitlist {
	#[inline]
	fn default() -> Self {
		Self {
			hostfile: PathBuf::from("/etc/hosts"),
			flags: 0,
			exclude: HashSet::default(),
			regexclude: None,
			found: HashSet::with_capacity(131_072),
			out: Vec::with_capacity(2_097_152),
		}
	}
}

impl fmt::Display for Shitlist {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// # Builder methods.
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
	///
	/// Also note that all regular expressions must be passed in a single call.
	/// This method always replaces what was there before.
	pub fn without_regex<I>(mut self, excludes: I) -> Self
	where I: IntoIterator<Item=String> {
		self.regexclude(excludes);
		self
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
	///
	/// ## Errors
	///
	/// This returns an error if any of the data sources could not be fetched
	/// or parsed, or if there are issues reading the hostfile.
	pub fn build(mut self) -> Result<Self, AdbyssError> {
		self.found.par_extend(Source::fetch_many(self.flags)?);

		// Post-processing.
		self.build_out()?;

		// We're done!
		Ok(self)
	}
}

/// # Setters.
impl Shitlist {
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
		self.found.extend(extras.into_iter().filter_map(Domain::new));
		let _res = self.build_out();
	}

	/// # Exclude Entries.
	///
	/// Exclude one or more arbitrary domains from the shitlist. This is
	/// primarily intended for cases where an authoritative source blackholes
	/// an address you want to be able to visit, e.g. `supportxmr.com`.
	pub fn exclude<I>(&mut self, excludes: I)
	where I: IntoIterator<Item=String> {
		self.exclude.extend(excludes.into_iter().filter_map(Domain::new));
	}

	/// # Exclude Entries (Regular Expression).
	///
	/// This is the same as [`Shitlist::exclude`] except it takes regular
	/// expressions.
	pub fn regexclude<I>(&mut self, excludes: I)
	where I: IntoIterator<Item=String> {
		self.regexclude = RegexSet::new(excludes)
			.ok()
			.filter(|re| ! re.is_empty());
	}
}

/// # Conversion.
impl Shitlist {
	#[allow(unsafe_code)]
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
	pub fn into_vec(self) -> Vec<String> {
		let mut found: Vec<String> = self.found.into_par_iter()
			.filter(|x| x.len() <= MAX_LINE)
			.map(Domain::take)
			.collect();
		found.par_sort();
		found
	}
}

/// # Details.
impl Shitlist {
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
}

/// # Misc.
impl Shitlist {
	/// # Stub.
	///
	/// Return the user portion of the specified hostfile.
	///
	/// ## Errors
	///
	/// This returns an error if there are issues reading or parsing the
	/// hostfile.
	fn hostfile_stub(&self) -> Result<String, AdbyssError> {
		use std::io::{
			BufRead,
			BufReader,
		};

		// Load existing hosts.
		let mut txt: String = String::with_capacity(512);
		let mut watermark: Watermark = Watermark::Zero;

		for line in File::open(&self.hostfile)
			.map(BufReader::new)
			.map_err(|_| AdbyssError::HostsRead(Box::from(self.hostfile.clone())))?
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
	///
	/// ## Errors
	///
	/// This returns an error if there are issues writing changes to the
	/// hostfile.
	pub fn unwrite(&self) -> Result<(), AdbyssError> {
		// Prompt about writing it?
		if
			0 == self.flags & FLAG_Y &&
			! confirm!(yes: format!(
				"Remove all Adbyss blackhole entries from {:?}?",
				&self.hostfile
			))
		{
			return Err(AdbyssError::Aborted);
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
	///
	/// ## Errors
	///
	/// This returns an error if there are issues writing changes to the
	/// hostfile.
	pub fn write(&self) -> Result<(), AdbyssError> {
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
	///
	/// ## Errors
	///
	/// This returns an error if there are issues writing changes to the
	/// hostfile.
	fn write_to<P>(&self, dst: P) -> Result<(), AdbyssError>
	where P: AsRef<Path> {
		let mut dst: PathBuf = dst.as_ref().to_path_buf();

		// Does it already exist?
		if dst.exists() {
			dst = dst.canonicalize()
				.ok()
				.filter(|x| ! x.is_dir())
				.ok_or_else(|| AdbyssError::HostsInvalid(Box::from(dst)))?;

			// Prompt about writing it?
			if
				0 == self.flags & FLAG_Y &&
				! confirm!(yes: format!(
					"Write {} hosts to {:?}?",
					NiceU64::from(self.len()).as_str(),
					dst
				))
			{
				return Err(AdbyssError::Aborted);
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
		if ! self.found.is_empty() {
			let extra = self.found
				.par_iter()
				.filter_map(Domain::without_www)
				.collect::<HashSet<Domain>>();
			self.found.par_extend(extra);
		}
	}

	/// # Backup.
	fn backup(&self, dst: &Path) -> Result<(), AdbyssError> {
		// Back it up!
		if 0 != self.flags & FLAG_BACKUP {
			// Tack ".adbyss.bak" onto the original path.
			let dst2 = PathBuf::from(OsStr::from_bytes(&[
				dst.as_os_str().as_bytes(),
				b".adbyss.bak"
			].concat()));

			// Copy the original, clobbering only as a fallback.
			std::fs::copy(dst, &dst2)
				.map_err(|_| AdbyssError::BackupWrite(Box::from(dst2)))?;
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
	fn build_out(&mut self) -> Result<(), AdbyssError> {
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
# Updated: {} UTC
# Blocked: {} garbage hosts
#
# Eat the rich.
#
##########
"#,
			FmtUtc2k::now().as_str(),
			NiceU64::from(self.found.len()).as_str()
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

	/// # Found: Compact.
	///
	/// This merges TLDs and their subdomains together to reduce the number of
	/// lines (and overall byte size), but without going overboard.
	fn found_compact(&self) -> Vec<String> {
		// Start by building up a map keyed by root domain...
		let mut found = HashMap::<u64, Vec<&Domain>, NoHash>::with_capacity_and_hasher(self.found.len(), NoHash::default());
		for dom in &self.found {
			let hash: u64 = hash64(dom.tld().as_bytes());
			found.entry(hash).or_insert_with(Vec::new).push(dom);
		}

		// Now build up each line.
		let mut found: Vec<String> = found.into_par_iter()
			.flat_map(|(_, mut x)| {
				// We have to split this into multiple lines so it can
				// fit.
				let mut out: Vec<String> = Vec::new();
				let mut line = String::new();

				// Split on whitespace.
				x.sort();
				for y in &x {
					if line.len() + 1 + y.len() <= MAX_LINE {
						if ! line.is_empty() {
							line.push(' ');
						}
						line.push_str(y);
					}
					else if ! line.is_empty() {
						out.push(line.split_off(0));
						if y.len() <= MAX_LINE {
							line.push_str(y);
						}
					}
				}

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
			.map(ToString::to_string)
			.collect();
		found.par_sort();
		found
	}

	#[allow(clippy::type_complexity)] // This is it.
	/// # Strip Ignores: Static Filter
	///
	/// Because this filter could run 60K times or more, it is worth taking
	/// a moment to optimize the matcher.
	fn strip_excludes_cb(&self) -> Option<Box<dyn Fn(&&Domain) -> bool + Send + Sync>> {
		match (&self.regexclude, 1.cmp(&self.exclude.len())) {
			// Neither.
			(None, Ordering::Greater) => None,
			// Only regexclude.
			(Some(re), Ordering::Greater) => {
				let re = re.clone();
				Some(Box::new(move |x| re.is_match(x)))
			},
			// Both, optimized static.
			(Some(re), Ordering::Equal) => {
				let re = re.clone();
				let val = self.exclude.iter().next().unwrap().clone();
				Some(Box::new(move |x| x == &&val || re.is_match(x)))
			},
			// Optimized static.
			(None, Ordering::Equal) => {
				let val = self.exclude.iter().next().unwrap().clone();
				Some(Box::new(move |x| x == &&val))
			},
			// Both, many statics.
			(Some(re), Ordering::Less) => {
				let re = re.clone();
				let ex = self.exclude.clone();
				Some(Box::new(move |x| re.is_match(x) || ex.contains(x)))
			},
			// Many statics.
			(None, Ordering::Less) => {
				let ex = self.exclude.clone();
				Some(Box::new(move |x| ex.contains(x)))
			},
		}
	}

	/// # Strip Ignores.
	///
	/// This removes any excluded domains from the results.
	fn strip_excludes(&mut self) {
		if self.found.is_empty() {
			return;
		}

		if let Some(cb) = self.strip_excludes_cb() {
			self.found.retain(|x| ! cb(&x));
		}
	}
}



#[inline]
/// # `AHash` Byte Hash.
///
/// This is a convenience method for quickly hashing bytes using the
/// [`AHash`](https://crates.io/crates/ahash) crate. Check out that project's
/// home page for more details. Otherwise, TL;DR it is very fast.
fn hash64(src: &[u8]) -> u64 {
	ahash::RandomState::with_seeds(13, 19, 23, 71).hash_one(src)
}


#[allow(unsafe_code)]
/// # Parse Custom Hosts.
///
/// This is used to parse custom hosts out of the user's `/etc/hosts` file.
/// We'll want to exclude these from the blackhole list to prevent duplicates,
/// however unlikely that may be.
fn parse_custom_hosts(raw: &str) -> HashSet<Domain> {
	raw.par_lines()
		.filter_map(|x| {
			// Split on whitespace, up to the first #comment, if any.
			let mut split = x.bytes()
				.position(|b| b'#' == b)
				.map_or(x, |p|
					if x.is_char_boundary(p) {
						unsafe { x.get_unchecked(0..p) }
					}
					else { "" }
				)
				.split_whitespace();

			// If the first entry is an IP address, parse all subsequent
			// entries as possible hosts.
			if split.next().and_then(|x| x.parse::<std::net::IpAddr>().ok()).is_some() {
				Some(split.filter_map(Domain::new).collect::<Vec<Domain>>())
			}
			else { None }
		})
		.flatten()
		.collect()
}

/// # Write Helper.
///
/// This method will first attempt an atomic write using `tempfile`, but if
/// that fails — as is common with `/etc/hosts` — it will try a nonatomic write
/// instead.
fn write_to_file(path: &Path, data: &[u8]) -> Result<(), AdbyssError> {
	use std::io::Write;

	// Try an atomic write first.
	write_atomic::write_file(path, data)
		.or_else(|_| File::create(path)
			.and_then(|mut file| file.write_all(data).and_then(|_| file.flush()))
		)
		.map_err(|_| AdbyssError::HostsWrite(Box::from(path)))
}

