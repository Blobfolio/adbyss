/*!
# Adbyss: Sources
*/

use crate::AdbyssError;
use std::{
	borrow::Cow,
	fs::File,
	path::{
		Path,
		PathBuf,
	},
};



#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Shitlist Source.
pub(super) enum Source {
	/// # Adaway.
	AdAway =      0b0001_u8,

	/// # Adbyss.
	Adbyss =      0b0010_u8,

	/// # Steven Black.
	StevenBlack = 0b0100_u8,

	/// # Yoyo.
	Yoyo =        0b1000_u8,
}

/// # Conversion.
impl Source {
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::AdAway => "AdAway",
			Self::Adbyss => "Adbyss",
			Self::StevenBlack => "Steven Black",
			Self::Yoyo => "Yoyo",
		}
	}
}

/// # Getters.
impl Source {
	/// # Cache path.
	fn cache_path(self) -> PathBuf {
		std::env::temp_dir().join(
			match self {
				Self::AdAway => "_adbyss-adaway.tmp",
				Self::Adbyss => "_adbyss.tmp",
				Self::StevenBlack => "_adbyss-sb.tmp",
				Self::Yoyo => "_adbyss-yoyo.tmp",
			}
		)
	}

	/// # Fetch Raw Source Data.
	///
	/// ## Errors
	///
	/// This returns an error if the data cannot be downloaded or parsed.
	pub(super) fn fetch_raw(self) -> Result<Cow<'static, str>, AdbyssError> {
		use std::io::Write;

		// Adbyss' own dataset is static.
		if matches!(self, Self::Adbyss) {
			return Ok(Cow::Borrowed(include_str!("../skel/adbyss.txt")));
		}

		// Check the cache first. If the source was downloaded less than an
		// hour ago, we can use that instead of asking the Internet for a new
		// copy.
		let cache = self.cache_path();
		if let Some(out) = read_from_cache(&cache) { return Ok(Cow::Owned(out)); }

		// Try to download it.
		let out = download_source(self)?;

		// Cache it for next time. If this doesn't work, we'll just have to
		// download it each time. Whatever.
		let _res = File::create(&cache).and_then(|mut file|
			file.write_all(out.as_bytes()).and_then(|()| file.flush())
		);

		// Return it!
		Ok(Cow::Owned(out))
	}

	/// # Line Prefix.
	pub(super) const fn line_prefix(self) -> &'static str {
		match self {
			Self::AdAway | Self::Yoyo => "127.0.0.1 ",
			Self::StevenBlack =>  "0.0.0.0 ",
			Self::Adbyss => "",
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



/// # Download Source.
///
/// This will try to fetch the remote source data, using Gzip encoding where
/// possible to reduce the transfer times. All sources currently serve Gzipped
/// content, so the extra complexity is worth it.
fn download_source(kind: Source) -> Result<String, AdbyssError> {
	if
		let Ok(res) = minreq::get(kind.url())
			.with_header("user-agent", "Mozilla/5.0")
			.with_timeout(15)
			.send() &&
		(200..=399).contains(&res.status_code) &&
		let Ok(out) = res.as_str()
	{
		Ok(out.to_owned())
	}
	else { Err(AdbyssError::SourceFetch(kind)) }
}

/// # Read From Cache.
///
/// Read and return the file, but only if it was modified within the past hour.
fn read_from_cache(src: &Path) -> Option<String> {
	let meta = std::fs::metadata(src).ok()?;
	if meta.is_file() {
		let elapsed = meta.modified().ok().and_then(|m| m.elapsed().ok())?;
		if elapsed.as_secs() < 3600 {
			return std::fs::read_to_string(src).ok();
		}
	}

	// Nope.
	None
}
