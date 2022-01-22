/*!
# `Adbyss`: Sources
*/

use adbyss_psl::Domain;
use crate::{
	AdbyssError,
	AHASH_STATE,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_STEVENBLACK,
	FLAG_YOUTUBE,
	FLAG_YOYO,
};
use rayon::{
	iter::{
		IntoParallelIterator,
		IntoParallelRefIterator,
		ParallelExtend,
		ParallelIterator,
	},
	prelude::ParallelString,
};
use std::{
	borrow::Cow,
	collections::HashSet,
	fs::File,
	path::PathBuf,
};



#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
/// # Shitlist Sources.
pub enum Source {
	/// AdAway.
	AdAway,
	/// Adbyss.
	Adbyss,
	/// StevenBlack.
	StevenBlack,
	/// Youtube.
	YouTube,
	/// Yoyo.
	Yoyo,
}

/// # Conversion.
impl Source {
	#[must_use]
	/// # As Byte (Flag).
	///
	/// Return the equivalent flag for the source.
	const fn as_byte(self) -> u8 {
		match self {
			Self::AdAway => FLAG_ADAWAY,
			Self::Adbyss => FLAG_ADBYSS,
			Self::StevenBlack => FLAG_STEVENBLACK,
			Self::YouTube => FLAG_YOUTUBE,
			Self::Yoyo => FLAG_YOYO,
		}
	}

	#[must_use]
	/// # As Str.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::AdAway => "AdAway",
			Self::Adbyss => "Adbyss",
			Self::StevenBlack => "Steven Black",
			Self::YouTube => "YouTube",
			Self::Yoyo => "Yoyo",
		}
	}
}

/// # Getters.
impl Source {
	#[must_use]
	/// # Cache path.
	fn cache_path(self) -> PathBuf {
		let mut out: PathBuf = std::env::temp_dir();
		out.push(
			match self {
				Self::AdAway => "_adbyss-adaway.tmp",
				Self::Adbyss => "_adbyss.tmp",
				Self::StevenBlack => "_adbyss-sb.tmp",
				Self::YouTube => "_adbyss-yt.tmp",
				Self::Yoyo => "_adbyss-yoyo.tmp",
			}
		);
		out
	}

	#[must_use]
	/// # Patch Raw.
	///
	/// Some sources return a straight domain list, while others are formatted
	/// more like a hosts file, with IP address prefixes, comments, etc. This
	/// method normalizes the data so that it is just a long list of domains.
	fn patch(self, src: String) -> String {
		let prefix: &str = match self {
			Self::AdAway | Self::Yoyo => "127.0.0.1 ",
			Self::StevenBlack =>  "0.0.0.0 ",
			// The YouTube and built-in lists are close enough as-are.
			_ => return src,
		};

		src.lines()
			.filter_map(|mut line| {
				// Strip the prefix.
				line = line.strip_prefix(prefix)?;

				// Strip comments.
				if let Some(pos) = line.bytes().position(|b| b == b'#') {
					line = &line[..pos];
				}

				// Return if we have something!
				line = line.trim();
				if line.is_empty() { None }
				else { Some(line) }
			})
			.collect::<Vec<&str>>()
			.join("\n")
	}

	#[must_use]
	/// # Source URL.
	///
	/// For remote hosts, return the URL where data is found.
	const fn url(self) -> &'static str {
		match self {
			Self::AdAway => "https://adaway.org/hosts.txt",
			Self::Adbyss => "",
			Self::StevenBlack => "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
			Self::YouTube => "https://raw.githubusercontent.com/Ewpratten/youtube_ad_blocklist/master/blocklist.txt",
			Self::Yoyo => "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
		}
	}
}

/// # Raw Data.
impl Source {
	/// # Fetch Raw Source Data.
	///
	/// ## Errors
	///
	/// This returns an error if the data cannot be downloaded or parsed.
	fn fetch_raw(self) -> Result<Cow<'static, str>, AdbyssError> {
		use std::io::Write;

		// Adbyss' own dataset is static.
		if self == Self::Adbyss {
			return Ok(Cow::Borrowed(include_str!("../skel/adbyss.txt")));
		}

		// Check the cache first. If the source was downloaded less than an
		// hour ago, we can use that instead of asking the Internet for a new
		// copy.
		let cache = self.cache_path();
		if let Some(x) = std::fs::metadata(&cache)
			.ok()
			.filter(std::fs::Metadata::is_file)
			.and_then(|meta| meta.modified().ok())
			.and_then(|time| time.elapsed()
				.ok()
				.filter(|secs| 3600 > secs.as_secs())
			)
			.and_then(|_| std::fs::read_to_string(&cache).ok())
		{
			return Ok(Cow::Owned(x));
		}

		// Try to download it.
		let out = download_source(self).map(|x| self.patch(x))?;

		// Cache it for next time. If this doesn't work, we'll just have to
		// download it each time. Whatever.
		let _res = File::create(&cache).and_then(|mut file|
			file.write_all(out.as_bytes()).and_then(|_| file.flush())
		);

		// Return it!
		Ok(Cow::Owned(out))
	}

	/// # Fetch Many Raw Source Data.
	///
	/// ## Errors
	///
	/// This returns an error if any source data could be downloaded or parsed.
	pub fn fetch_many(src: u8) -> Result<HashSet<Domain, ahash::RandomState>, AdbyssError> {
		// First, build a consolidated string of all the entries.
		let raw: Cow<str> = [Self::AdAway, Self::Adbyss, Self::StevenBlack, Self::YouTube, Self::Yoyo].par_iter()
				.filter(|x| 0 != src & x.as_byte())
				.map(|x| x.fetch_raw())
				// Merge the raw data into a single block so we can better
				// parallelize parsing. If any sources failed, operations will
				// abort here.
				.try_reduce(Cow::default, |mut a, b| {
					let tmp = a.to_mut();
					if ! tmp.is_empty() {
						tmp.push('\n');
					}
					tmp.push_str(b.trim());
					Ok(a)
				})?;

		// There are a lot of duplicates, so let's quickly weed them out before
		// we move onto the relatively expensive domain validation checks.
		let mut tmp: HashSet<&str, ahash::RandomState> = HashSet::with_capacity_and_hasher(100_000, AHASH_STATE);
		tmp.par_extend(raw.par_lines());

		// And finally, see which of those lines are actual domains.
		let mut out: HashSet<Domain, ahash::RandomState> = HashSet::with_capacity_and_hasher(tmp.len(), AHASH_STATE);
		out.par_extend(tmp.into_par_iter().filter_map(Domain::new));
		Ok(out)
	}
}



/// # Download Source.
///
/// This will try to fetch the remote source data, using Gzip encoding where
/// possible to reduce the transfer times. All sources currently serve Gzipped
/// content, so the extra complexity is worth it.
fn download_source(kind: Source) -> Result<String, AdbyssError> {
	ureq::get(kind.url())
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(std::convert::Into::into))
		.map_err(|_| AdbyssError::SourceFetch(kind))
}
