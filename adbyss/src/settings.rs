/*!
# `Adbyss` - Settings
*/

use adbyss_core::{
	Shitlist,
	FLAG_ADAWAY,
	FLAG_ADBYSS,
	FLAG_ALL,
	FLAG_BACKUP,
	FLAG_STEVENBLACK,
	FLAG_YOYO,
};
use fyi_msg::MsgKind;
use serde::Deserialize;
use std::path::PathBuf;



#[allow(clippy::redundant_pub_crate)] // Clippy is confused.
#[allow(clippy::struct_excessive_bools)] // This is coming from Yaml.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
/// # Settings.
pub(crate) struct Settings {
	#[serde(default = "Settings::config")]
	hostfile: PathBuf,

	#[serde(default = "Settings::source_default")]
	source_adaway: bool,

	#[serde(default = "Settings::source_default")]
	source_adbyss: bool,

	#[serde(default = "Settings::source_default")]
	source_stevenblack: bool,

	#[serde(default = "Settings::source_default")]
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

impl From<PathBuf> for Settings {
	fn from(src: PathBuf) -> Self {
		if src.is_file() {
			if let Ok(txt) = std::fs::read_to_string(&src) {
				if let Ok(tmp) = serde_yaml::from_str::<Self>(&txt) {
					return tmp;
				}
			}

			MsgKind::Error
				.into_msg(&format!("Unable to parse configuration: {:?}", src))
				.eprintln();
		}
		else {
			MsgKind::Error
				.into_msg(&format!("Missing configuration: {:?}", src))
				.eprintln();
		}

		std::process::exit(1);
	}
}

impl Settings {
	/// # Configuration Path.
	pub(crate) fn config() -> PathBuf { PathBuf::from("/etc/adbyss.yaml") }

	/// # Source Default.
	///
	/// This is used to provide Serde with a "true" value. Not sure how to do
	/// that using their macro. Haha.
	pub(crate) const fn source_default() -> bool { true }

	/// # Into Shitlist.
	pub(crate) fn into_shitlist(self) -> Shitlist {
		// Note: the backup CLI flag is the opposite of the constant, so we'll
		// start with it in place and `main()` will subtract if needed.
		let mut flags: u8 = FLAG_BACKUP | FLAG_ALL;

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



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_filters() {
		let shitlist = Settings::from(PathBuf::from("./misc/test.yaml"))
			.into_shitlist()
			.build()
			.unwrap();
		let res = shitlist.as_str();

		// Our includes should be present.
		assert!(res.contains("batman.com"));
		assert!(res.contains("spiderman.com"));

		// Our excludes should not be present.
		assert!(! res.contains("collect.snitcher.com"));
		assert!(! res.contains("triptease.io"));

		// Adbyss' other entries should still be there.
		assert!(res.contains("www.snitcher.com"));
	}
}
