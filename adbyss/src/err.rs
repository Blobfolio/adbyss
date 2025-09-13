/*!
# Adbyss: Errors
*/

use crate::Source;
use fyi_msg::fyi_ansi::{
	ansi,
	csi,
	dim,
};
use std::fmt;



/// # Help Text.
const HELP: &str = concat!(r#"
 .--,       .--,
( (  \.---./  ) )
 '.__/o   o\__.'
    (=  ^  =)       "#, csi!(199), "Adbyss", ansi!((cornflower_blue) " v", env!("CARGO_PKG_VERSION")), r#"
     >  -  <        Block ads, trackers, malware, and
    /       \       other garbage sites in /etc/hosts.
   //       \\
  //|   .   |\\
  "'\       /'"_.-~^`'-.
     \  _  /--'         `
   ___)( )(___

USAGE:
    adbyss [FLAGS] [OPTIONS]

FLAGS:
        --disable      Remove *all* Adbyss entries from the hostfile.
    -h, --help         Prints help information.
    -q, --quiet        Do *not* summarize changes after write.
        --show         Print a sorted blackholable hosts list to STDOUT, one per
                       line.
        --stdout       Print the would-be hostfile to STDOUT instead of writing
                       it to disk.
    -V, --version      Prints version information.
    -y, --yes          Non-interactive mode; answer "yes" to all prompts.

OPTIONS:
    -c, --config <path>    Use this configuration instead of /etc/adbyss.toml.

SOURCES:
    AdAway:       <https://adaway.org/>
    Steven Black: <https://github.com/StevenBlack/hosts>
    Yoyo:         <https://pgl.yoyo.org/adservers/>

Additional global settings are stored in /etc/adbyss.toml.
"#);



#[derive(Debug, Clone)]
/// # Error.
pub(super) enum AdbyssError {
	/// # Early Abort.
	Aborted,

	/// # Invalid CLI Argument.
	InvalidCli(String),

	/// # No Internet.
	NoInternet,

	/// # No Shitlist.
	NoShitlist,

	/// # Deserialization Error.
	Parse(String),

	/// # Root required.
	Root,

	/// # Read Issue.
	Read(String),

	/// # Unable to Fetch Source.
	SourceFetch(Source),

	/// # Write Issue.
	Write(String),

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl std::error::Error for AdbyssError {}

impl fmt::Display for AdbyssError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())?;
		match self {
			Self::InvalidCli(s) | Self::Parse(s) | Self::Read(s) | Self::Write(s) =>
				write!(f, dim!(" ({})"), s),
			Self::SourceFetch(s) => write!(f, dim!(" ({})"), s.as_str()),
			_ => Ok(()),
		}
	}
}

impl AdbyssError {
	/// # As String Slice.
	pub(super) const fn as_str(&self) -> &'static str {
		match self {
			Self::Aborted => "Operation aborted.",
			Self::InvalidCli(_) => "Invalid/unknown option.",
			Self::NoInternet => "No internet connection available.",
			Self::NoShitlist => "There are no domains to blackhole!",
			Self::Parse(_) => "Parsing failed.",
			Self::Read(_) => "Unable to read file.",
			Self::Root => "Adbyss requires root privileges.",
			Self::SourceFetch(_) => "Unable to fetch source.",
			Self::Write(_) => "Unable to write file.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("Adbyss v", env!("CARGO_PKG_VERSION")),
		}
	}
}
