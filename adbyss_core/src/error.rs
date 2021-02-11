/*!
# Adbyss: Block List Errors
*/


use crate::Source;
use fyi_menu::ArgueError;
use std::{
	error::Error,
	fmt,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # Error.
pub enum AdbyssError {
	/// # Aborted.
	Aborted,
	/// # Menu error.
	Argue(ArgueError),
	/// # Backup write.
	BackupWrite(PathBuf),
	/// # Invalid configuration.
	Config(PathBuf),
	/// # Invalid Hosts.
	HostsInvalid(PathBuf),
	/// # Read error.
	HostsRead(PathBuf),
	/// # Write error.
	HostsWrite(PathBuf),
	/// # Root required.
	Root,
	/// # Fetching source failed.
	SourceFetch(Source),
}

impl Error for AdbyssError {}

impl fmt::Display for AdbyssError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Aborted => f.write_str("Operation aborted."),
			Self::Argue(src) => f.write_str(src.as_ref()),
			Self::BackupWrite(path) => f.write_fmt(format_args!("Unable to write backup: {:?}", path)),
			Self::Config(path) => f.write_fmt(format_args!("Invalid configuration: {:?}", path)),
			Self::HostsInvalid(path) => f.write_fmt(format_args!("Invalid hostfile: {:?}", path)),
			Self::HostsRead(path) => f.write_fmt(format_args!("Unable to read hostfile: {:?}", path)),
			Self::HostsWrite(path) => f.write_fmt(format_args!("Unable to write hostfile: {:?}", path)),
			Self::Root => f.write_str("Adbyss requires root privileges."),
			Self::SourceFetch(src) => f.write_fmt(format_args!("Unable to fetch source: {}", src.as_str())),
		}
	}
}
