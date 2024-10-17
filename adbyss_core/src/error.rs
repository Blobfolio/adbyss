/*!
# Adbyss: Block List Errors
*/


use crate::Source;
use std::{
	error::Error,
	fmt,
	path::Path,
};



#[derive(Debug, Clone)]
/// # Error.
pub enum AdbyssError {
	/// # Aborted.
	Aborted,

	/// # Backup write.
	BackupWrite(Box<Path>),

	/// # Invalid configuration.
	Config(Box<Path>),

	/// # Invalid Hosts.
	HostsInvalid(Box<Path>),

	/// # Read error.
	HostsRead(Box<Path>),

	/// # Write error.
	HostsWrite(Box<Path>),

	/// # Uknown CLI option.
	InvalidCli(Box<str>),

	/// # No Internet.
	NoInternet,

	/// # Root required.
	Root,

	/// # Fetching source failed.
	SourceFetch(Source),

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl Error for AdbyssError {}

impl fmt::Display for AdbyssError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Aborted => f.write_str("Operation aborted."),
			Self::BackupWrite(path) => f.write_fmt(format_args!("Unable to write backup: {path:?}")),
			Self::Config(path) => f.write_fmt(format_args!("Invalid configuration: {path:?}")),
			Self::HostsInvalid(path) => f.write_fmt(format_args!("Invalid hostfile: {path:?}")),
			Self::HostsRead(path) => f.write_fmt(format_args!("Unable to read hostfile: {path:?}")),
			Self::HostsWrite(path) => f.write_fmt(format_args!("Unable to write hostfile: {path:?}")),
			Self::InvalidCli(s) => f.write_fmt(format_args!("Invalid/unknown option: {s}")),
			Self::NoInternet => f.write_str("No internet connection was available."),
			Self::Root => f.write_str("Adbyss requires root privileges."),
			Self::SourceFetch(src) => f.write_fmt(format_args!("Unable to fetch source: {}", src.as_str())),
			Self::PrintHelp | Self::PrintVersion => Ok(()),
		}
	}
}
