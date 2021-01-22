/*!
# `Adbyss` - Settings
*/

use adbyss_core::{
	AdbyssError,
	Shitlist,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_ALL,
	FLAG_BACKUP,
	FLAG_STEVENBLACK,
	FLAG_YOYO,
	FLAG_COMPACT,
};
use serde::Deserialize;
use std::{
	convert::TryFrom,
	path::PathBuf,
};



#[allow(clippy::struct_excessive_bools)] // This is coming from Yaml.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
/// # Settings.
pub(super) struct Settings {
	#[serde(default = "Settings::config")]
	hostfile: PathBuf,

	#[serde(default = "default_true")]
	backup: bool,

	#[serde(default = "default_false")]
	compact: bool,

	#[serde(default = "default_true")]
	source_adaway: bool,

	#[serde(default = "default_true")]
	source_adbyss: bool,

	#[serde(default = "default_true")]
	source_stevenblack: bool,

	#[serde(default = "default_true")]
	source_yoyo: bool,

	#[serde(default = "Vec::new")]
	exclude: Vec<String>,

	#[serde(default = "Vec::new")]
	regexclude: Vec<String>,

	#[serde(default = "Vec::new")]
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
			.ok_or(AdbyssError::Config(path))
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
		if ! self.backup { flags &= ! FLAG_BACKUP }
		if self.compact { flags |= FLAG_COMPACT; }

		// Remove any disabled sources.
		if ! self.source_adbyss { flags &= ! FLAG_ADBYSS; }
		if ! self.source_adaway { flags &= ! FLAG_ADAWAY; }
		if ! self.source_stevenblack { flags &= ! FLAG_STEVENBLACK; }
		if ! self.source_yoyo { flags &= ! FLAG_YOYO; }

		// And build!
		Shitlist::default()
			.with_flags(flags)
			.with_hostfile(self.hostfile)
			.without(self.exclude)
			.without_regex(self.regexclude)
			.with(self.include)
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
		let shitlist = Settings::try_from(PathBuf::from("./skel/test.yaml"))
			.expect("Missing settings.")
			.into_shitlist()
			.build()
			.unwrap();
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
