/*!
# Adbyss: Settings
*/

use adbyss_psl::Domain;
use crate::{
	AdbyssError,
	MAX_LINE,
	Shitlist,
	Source,
};
use dactyl::NiceU64;
use regex::RegexSet;
use serde::{
	de,
	Deserialize,
};
use std::{
	borrow::Cow,
	collections::BTreeSet,
	path::{
		Path,
		PathBuf,
	},
	str::Lines,
};



#[expect(clippy::struct_excessive_bools, reason = "The fields mirror our YAML config.")]
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
/// # Settings.
///
/// This struct holds the user's runtime preferences.
pub(super) struct Settings {
	/// # Hosts File Path.
	hostfile: PathBuf,

	/// # Backup Original Hosts?
	backup: bool,

	/// # Join Hosts by TLD?
	compact: bool,

	/// # Use Adaway Sources?
	source_adaway: bool,

	/// # Use Adbyss Sources?
	source_adbyss: bool,

	/// # Use Steven Black Sources?
	source_stevenblack: bool,

	/// # Use Yoyo Sources?
	source_yoyo: bool,

	/// # Domains to Exclude.
	exclude: BTreeSet<Domain>,

	#[serde(deserialize_with = "deserialize_regexclude")]
	/// # Patterns to Exclude.
	regexclude: Option<RegexSet>,

	#[serde(deserialize_with = "deserialize_include")]
	/// # Domains to Include.
	include: Vec<String>,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			hostfile: PathBuf::from(Self::DEFAULT_HOSTFILE),
			backup: true,
			compact: false,
			source_adaway: true,
			source_adbyss: true,
			source_stevenblack: true,
			source_yoyo: true,
			exclude: BTreeSet::new(),
			regexclude: None,
			include: Vec::new(),
		}
	}
}

impl Settings {
	/// # Default Hostfile.
	pub(super) const DEFAULT_HOSTFILE: &str = "/etc/hosts";

	/// # Default Config Location.
	pub(super) const DEFAULT_CONFIG: &str = "/etc/adbyss.yaml";

	/// # From File.
	pub(super) fn from_file<P: AsRef<Path>>(src: P) -> Result<Self, AdbyssError> {
		let src = src.as_ref();
		let raw = std::fs::read_to_string(src)
			.map_err(|_| AdbyssError::Read(src.to_string_lossy().into_owned()))?;
		serde_yml::from_str::<Self>(&raw)
			.map_err(|e| AdbyssError::Parse(e.to_string()))
	}
}

impl Settings {
	/// # Backup Original?
	pub(super) const fn backup(&self) -> bool { self.backup }

	/// # Compact Output?
	pub(super) const fn compact(&self) -> bool { self.compact }

	/// # Needs Internet?
	pub(super) const fn needs_internet(&self) -> bool {
		self.source_adaway || self.source_stevenblack || self.source_yoyo
	}
}

impl Settings {
	/// # Build Hosts File.
	///
	/// Build and return the hosts file content and entry count _without_
	/// saving it anywhere.
	///
	/// ## Errors
	///
	/// This will bubble up any errors encountered along the way.
	pub(super) fn build(&self) -> Result<(String, usize), AdbyssError> {
		// Pull the current hosts file, stripped of any previous adbyss stuff.
		let (mut out, _) = crate::write::read_hosts(&self.hostfile)?;

		// Pull the shitlist and parse-and-prune the custom hosts from above
		// out of the list.
		let mut shitlist = self.shitlist()?;
		shitlist.prune_custom_hosts(&out);

		let len = shitlist.len();
		if len != 0 {
			shitlist.append(&mut out)
				.map_err(|_| AdbyssError::Write(self.hostfile.to_string_lossy().into_owned()))?;
		}

		Ok((out, len))
	}

	/// # Write Changes!
	///
	/// Update the hostsfile and return the number of domains written.
	///
	/// ## Errors
	///
	/// This will bubble up any errors encountered along the way.
	pub(super) fn write(&self, yes: bool) -> Result<usize, AdbyssError> {
		let (out, len) = self.build()?;

		// Double-check with the user before continuing.
		if ! yes && ! fyi_msg::confirm!(yes: format!(
			"Write {} hosts to {}?",
			NiceU64::from(len),
			self.hostfile.to_string_lossy(),
		)) {
			return Err(AdbyssError::Aborted);
		}

		// Backup and/or save.
		self.try_backup()?;
		crate::write::write_to_file(&self.hostfile, out.as_bytes()).map(|()| len)
	}

	/// # Unwrite Changes.
	///
	/// Remove Adbyss from the hosts file.
	///
	/// ## Errors
	///
	/// This will bubble up any errors encountered along the way.
	pub(super) fn unwrite(&self, yes: bool) -> Result<(), AdbyssError> {
		// Pull the current hosts file, stripped of any previous adbyss stuff.
		let (out, changed) = crate::write::read_hosts(&self.hostfile)?;

		// We only need to take action if there were entries to begin with.
		if changed {
			// Prompt the user before taking any action.
			if ! yes && ! fyi_msg::confirm!(yes: format!(
				"Remove all Adbyss blackhole entries from {}?",
				self.hostfile.to_string_lossy(),
			)) {
				return Err(AdbyssError::Aborted);
			}

			self.try_backup()?;
			crate::write::write_to_file(&self.hostfile, out.as_bytes())?;
		}

		Ok(())
	}

	/// # Try Backup.
	///
	/// If backups are enabled and the hostfile exists, try to make a copy of
	/// it.
	///
	/// ## Errors
	///
	/// This will return an error if the write fails.
	fn try_backup(&self) -> Result<(), AdbyssError> {
		if self.backup() && self.hostfile.is_file() {
			let mut dst = self.hostfile.clone();
			dst.as_mut_os_string().push(".adbyss.bak");
			std::fs::copy(&self.hostfile, &dst)
				.map_err(|_| AdbyssError::Write(dst.to_string_lossy().into_owned()))?;
		}

		Ok(())
	}
}

impl Settings {
	/// # The Shitlist.
	///
	/// Fetch, crunch, and merge all enabled third-party lists and user
	/// includes, filter out the user excludes, sort, dedupe, and return!
	///
	/// ## Errors
	///
	/// This will only return an error if there's a problem fetching the
	/// source(s).
	pub(super) fn shitlist(&self) -> Result<Shitlist, AdbyssError> {
		let lists = self.download()?;

		// First, let's collect all domain-like string slices from the lists
		// as there are likely to be a lot of repeats.
		let mut raw: Vec<&str> = lists.iter()
			.flat_map(|(source, list)| SourceDomains {
				lines: list.lines(),
				prefix: source.line_prefix(),
				buf: None,
			})
			.chain(self.include.iter().map(String::as_str))
			.collect();
		raw.sort_unstable();
		raw.dedup();

		// With that out of the way, let's collect the _actual_ domains!
		let mut out: Vec<Domain> = raw.into_iter()
			.filter_map(|d| Domain::new(d).filter(|d| d.len() <= MAX_LINE))
			.collect();

		// Sort and dedupe again.
		out.sort_unstable();
		out.dedup();

		// Apply the user's exclude rules, if any.
		for ex in &self.exclude {
			if let Ok(pos) = out.binary_search(ex) { out.remove(pos); }
		}
		if let Some(re) = &self.regexclude { out.retain(|v| ! re.is_match(v)); }

		// Done!
		if self.compact() { Ok(Shitlist::Compact(out)) }
		else { Ok(Shitlist::Flat(out)) }
	}

	/// # Download.
	///
	/// Download (or pull from cache) all enabled source lists.
	fn download(&self) -> Result<[(Source, Cow<'static, str>); 4], AdbyssError> {
		let mut out = [
			(Source::AdAway, Cow::Borrowed("")),
			(Source::Adbyss, Cow::Borrowed("")),
			(Source::StevenBlack, Cow::Borrowed("")),
			(Source::Yoyo, Cow::Borrowed("")),
		];

		std::thread::scope(|s| {
			// Network I/O drags; let's parallelize our efforts!
			let workers = [
				(if self.source_adaway { Some(s.spawn(|| Source::AdAway.fetch_raw())) } else { None }),
				(if self.source_adbyss { Some(s.spawn(|| Source::Adbyss.fetch_raw())) } else { None }),
				(if self.source_stevenblack { Some(s.spawn(|| Source::StevenBlack.fetch_raw())) } else { None }),
				(if self.source_yoyo { Some(s.spawn(|| Source::Yoyo.fetch_raw())) } else { None }),
			];

			// Pull in the results.
			for (mut thread, (source, raw)) in workers.into_iter().zip(out.iter_mut()) {
				if let Some(thread) = thread.take() {
					let thread = thread.join()
						.map_err(|_| AdbyssError::SourceFetch(*source))?;
					*raw = thread?;
				}
			}

			// Done!
			Ok(out)
		})
	}
}



/// # Source Domains Iter.
///
/// Tease domain-like strings out of a raw source list.
struct SourceDomains<'a> {
	/// # Line Iterator.
	lines: Lines<'a>,

	/// # Leading IP to strip, if any.
	prefix: &'static str,

	/// # Non-www Buffer.
	buf: Option<&'a str>,
}

impl<'a> Iterator for SourceDomains<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			// Prioritize the buffer.
			if let Some(last) = self.buf.take() { return Some(last); }

			// Trim the line.
			let mut line = self.lines.next()?.trim();
			if line.is_empty() { continue; }

			// Depending on the source, we might need to strip an IP.
			if ! self.prefix.is_empty() {
				if let Some(rest) = snip_domain_line(line, self.prefix) {
					line = rest;
				}
				else { continue; }
			}

			// If anything remains, return it!
			if ! line.is_empty() {
				// If www is banned, non-www should be too.
				if let Some(rest) = line.strip_prefix("www.") {
					self.buf.replace(rest);
				}

				return Some(line);
			}
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let (_, hint) = self.lines.size_hint();
		(usize::from(self.buf.is_some()), hint)
	}
}



#[expect(clippy::unnecessary_wraps, reason = "We don't control the signature.")]
#[expect(clippy::option_if_let_else, reason = "Too messy.")]
/// # Deserialize Include.
fn deserialize_include<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where D: de::Deserializer<'de> {
	if let Ok(mut tmp) = Vec::<String>::deserialize(deserializer) {
		// Make sure the entries are lowercase, and if there are www-prefixed
		// domains, add their non-www counterparts.
		let mut extra = Vec::with_capacity(tmp.len());
		for v in &mut tmp {
			v.make_ascii_lowercase();
			if let Some(rest) = v.strip_prefix("www.") { extra.push(rest.to_owned()); }
		}
		tmp.append(&mut extra);

		// Sort and dedupe.
		tmp.sort_unstable();
		tmp.dedup();

		Ok(tmp)
	}
	else { Ok(Vec::new()) }
}

/// # Deserialize Regexclude.
fn deserialize_regexclude<'de, D>(deserializer: D) -> Result<Option<RegexSet>, D::Error>
where D: de::Deserializer<'de> {
	if let Ok(tmp) = Vec::<Cow<str>>::deserialize(deserializer) {
		let tmp = RegexSet::new(tmp).map_err(de::Error::custom)?;
		if tmp.is_empty() { Ok(None) }
		else { Ok(Some(tmp)) }
	}
	else { Ok(None) }
}

/// # Snip Domain Line.
///
/// Strip the given IP prefix from the start and any comments/whitespace from
/// the end. What's left should be a domain!
fn snip_domain_line<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
	let mut line = line.strip_prefix(prefix)?.trim_start();
	if let Some(pos) = line.find(|c: char| c == '#' || c.is_whitespace()) {
		line = &line[..pos];
	}
	Some(line)
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_filters() {
		let settings = Settings::from_file("skel/test.yaml")
			.expect("Unable to parse settings.");

		// The only enabled source should be our own (local) one.
		assert!(! settings.source_adaway);
		assert!(settings.source_adbyss);
		assert!(! settings.source_stevenblack);
		assert!(! settings.source_yoyo);

		// Parse the list.
		let res: Vec<String> = settings.shitlist()
			.expect("Shitlist failed!")
			.into_vec()
			.into_iter()
			.map(Domain::take)
			.collect();

		// Make sure our manual includes are present.
		assert!(res.contains(&String::from("batman.com")));
		assert!(res.contains(&String::from("spiderman.com")));

		// And our manual excludes are not.
		assert!(! res.contains(&String::from("collect.snitcher.com")));
		assert!(! res.contains(&String::from("triptease.io")));

		// Double check at least one of Adbyss' other entries is present.
		assert!(res.contains(&String::from("www.snitcher.com")));
	}
}
