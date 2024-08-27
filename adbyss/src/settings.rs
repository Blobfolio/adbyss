/*!
# `Adbyss` - Settings
*/

use adbyss_core::{
	AdbyssError,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_ALL,
	FLAG_BACKUP,
	FLAG_COMPACT,
	FLAG_STEVENBLACK,
	FLAG_YOYO,
	Shitlist,
};
use serde::Deserialize;
use std::path::PathBuf;



#[expect(clippy::struct_excessive_bools, reason = "The fields mirror our YAML config.")]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
/// # Settings.
pub(super) struct Settings {
	#[serde(default = "Settings::config")]
	/// # Hosts File Path.
	hostfile: PathBuf,

	#[serde(default = "default_true")]
	/// # Backup Original Hosts?
	backup: bool,

	#[serde(default = "default_false")]
	/// # Join Hosts by TLD?
	compact: bool,

	#[serde(default = "default_true")]
	/// # Use Adaway Sources?
	source_adaway: bool,

	#[serde(default = "default_true")]
	/// # Use Adbyss Sources?
	source_adbyss: bool,

	#[serde(default = "default_true")]
	/// # Use Steven Black Sources?
	source_stevenblack: bool,

	#[serde(default = "default_true")]
	/// # Use Yoyo Sources?
	source_yoyo: bool,

	#[serde(default = "Vec::new")]
	/// # Domains to Exclude.
	exclude: Vec<String>,

	#[serde(default = "Vec::new")]
	/// # Patterns to Exclude.
	regexclude: Vec<String>,

	#[serde(default = "Vec::new")]
	/// # Domains to Include.
	include: Vec<String>,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			hostfile: PathBuf::from("/etc/hosts"),
			backup: true,
			compact: false,
			source_adaway: true,
			source_adbyss: true,
			source_stevenblack: true,
			source_yoyo: true,
			exclude: Vec::new(),
			regexclude: Vec::new(),
			include: Vec::new(),
		}
	}
}

impl TryFrom<PathBuf> for Settings {
	type Error = AdbyssError;

	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		std::fs::canonicalize(&path)
			.and_then(std::fs::read_to_string)
			.ok()
			.and_then(|x| serde_yaml::from_str::<Self>(&x).ok())
			.ok_or_else(|| AdbyssError::Config(Box::from(path)))
	}
}

impl Settings {
	/// # Configuration Path.
	pub(super) fn config() -> PathBuf { PathBuf::from("/etc/adbyss.yaml") }

	/// # Into Shitlist.
	pub(super) fn into_shitlist(self) -> Shitlist {
		// Note: the backup CLI flag is the opposite of the constant, so we'll
		// start with it in place and `main()` will subtract if needed.
		let mut flags: u8 = FLAG_BACKUP | FLAG_ALL;

		// Other settings.
		if ! self.backup { flags &= ! FLAG_BACKUP; }
		if self.compact { flags |= FLAG_COMPACT; }

		// Remove any disabled sources.
		if ! self.source_adbyss { flags &= ! FLAG_ADBYSS; }
		if ! self.source_adaway { flags &= ! FLAG_ADAWAY; }
		if ! self.source_stevenblack { flags &= ! FLAG_STEVENBLACK; }
		if ! self.source_yoyo { flags &= ! FLAG_YOYO; }

		// And build!
		let mut out = Shitlist::default()
			.with_flags(flags)
			.with_hostfile(self.hostfile);

		if ! self.exclude.is_empty() {
			out.exclude(self.exclude);
		}

		if ! self.regexclude.is_empty() {
			out.regexclude(self.regexclude);
		}

		if ! self.include.is_empty() {
			out.include(self.include);
		}

		out
	}
}

/// # Default true.
const fn default_true() -> bool { true }

/// # Default false.
const fn default_false() -> bool { false }



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_filters() {
		let path = std::fs::canonicalize("./skel/test.yaml")
			.expect("Missing test.yaml");

		let shitlist = Settings::try_from(path)
			.expect("Missing settings.")
			.into_shitlist()
			.build()
			.expect("Unable to parse settings");

		let res = shitlist.into_vec();

		// Our includes should be present.
		assert!(res.contains(&String::from("batman.com")));
		assert!(res.contains(&String::from("spiderman.com")));

		// Our excludes should not be present.
		assert!(! res.contains(&String::from("collect.snitcher.com")));
		assert!(! res.contains(&String::from("triptease.io")));

		// Adbyss' other entries should still be there.
		assert!(res.contains(&String::from("www.snitcher.com")));
	}
}
