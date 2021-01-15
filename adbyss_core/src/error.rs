/*!
# Adbyss: Block List Errors
*/


use crate::ShitlistSource;
use std::{
	fmt,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # Error.
pub enum AdbyssError {
	/// # Aborted.
	Aborted,
	/// # Backup write.
	BackupWrite(PathBuf),
	/// # Invalid Hosts.
	HostsInvalid(PathBuf),
	/// # Read error.
	HostsRead(PathBuf),
	/// # Write error.
	HostsWrite(PathBuf),
	/// # Fetching source failed.
	SourceFetch(ShitlistSource),
}

impl fmt::Display for AdbyssError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Aborted => f.write_str("Operation aborted."),
			Self::BackupWrite(path) => f.write_fmt(format_args!("Unable to write backup: {:?}", path)),
			Self::HostsInvalid(path) => f.write_fmt(format_args!("Invalid hostfile: {:?}", path)),
			Self::HostsRead(path) => f.write_fmt(format_args!("Unable to read hostfile: {:?}", path)),
			Self::HostsWrite(path) => f.write_fmt(format_args!("Unable to write hostfile: {:?}", path)),
			Self::SourceFetch(src) => f.write_fmt(format_args!("Unable to fetch source: {}", src.as_str())),
		}
	}
}
