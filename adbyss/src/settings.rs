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
	FLAG_SUMMARIZE,
	FLAG_YOYO,
};
use fyi_msg::MsgKind;
use serde::Deserialize;
use std::path::PathBuf;



#[allow(clippy::redundant_pub_crate)] // Clippy is confused.
#[allow(clippy::struct_excessive_bools)] // This is coming from Yaml.
#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
/// # Settings.
pub(crate) struct Settings {
	hostfile: PathBuf,
	source_adaway: bool,
	source_adbyss: bool,
	source_stevenblack: bool,
	source_yoyo: bool,
	exclude: Vec<String>,
	regexclude: Vec<String>,
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
		}

		MsgKind::Error
			.into_msg(&format!("Unable to parse configuration: {:?}", src))
			.eprintln();
		std::process::exit(1);
	}
}

impl Settings {
	/// # Configuration Path.
	pub(crate) fn config() -> PathBuf { PathBuf::from("/etc/adbyss.yaml") }

	/// # Into Shitlist.
	pub(crate) fn into_shitlist(self) -> Shitlist {
		// Note: the CLI flags for summaries and backups are inverted (--no-*)
		// from the constants, so we'll start with them turned on. `Main` will
		// remove them if needed.
		let mut flags: u8 = FLAG_SUMMARIZE | FLAG_BACKUP | FLAG_ALL;

		// Convert our sources to flags.
		if ! self.source_adbyss { flags &= ! FLAG_ADBYSS; }
		if ! self.source_adaway { flags &= ! FLAG_ADAWAY; }
		if ! self.source_stevenblack { flags &= ! FLAG_STEVENBLACK; }
		if ! self.source_yoyo { flags &= ! FLAG_YOYO; }

		Shitlist::default()
			.with_flags(flags)
			.with_hostfile(self.hostfile)
			.without(self.exclude)
			.without_regex(self.regexclude)
			.with(self.include)
	}
}
