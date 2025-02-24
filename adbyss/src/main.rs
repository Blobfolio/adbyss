/*!
# Adbyss
*/

#![forbid(unsafe_code)]

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]



mod err;
mod settings;
mod source;
mod write;

use err::AdbyssError;
use settings::Settings;
use source::Source;
use write::Shitlist;

use argyle::Argument;
use fyi_msg::Msg;
use dactyl::NiceU64;
use std::{
	io::Write,
	process::Command,
};



/// # CLI: Disable Shitlist.
const CLI_DISABLE: u8 = 0b0000_0001;

/// # CLI: Quiet.
const CLI_QUIET: u8 =   0b0000_0010;

/// # CLI: Show Only.
const CLI_SHOW: u8 =    0b0000_0100;

/// # CLI: To STDOUT.
const CLI_STDOUT: u8 =  0b0000_1000;

/// # CLI: Systemd Mode.
const CLI_SYSTEMD: u8 = 0b0011_0000; // Implies yes.

/// # CLI: Yes to Prompts.
const CLI_YES: u8 =     0b0010_0000;

/// # Maximum Host Line.
///
/// The true limit is `256`; this adds a little padding for `0.0.0.0` and
/// whitespace.
const MAX_LINE: usize = 245;


/// Main.
fn main() {
	match main__() {
		Err(e @ (AdbyssError::PrintHelp | AdbyssError::PrintVersion)) => {
			println!("{e}");
		},
		Err(e) => { Msg::error(e.to_string()).die(1); },
		Ok(()) => {},
	}
}

#[inline]
/// Actual Main.
fn main__() -> Result<(), AdbyssError> {
	// We need root!
	require_root()?;

	// Set up the parser.
	let args = argyle::args()
		.with_keywords(include!(concat!(env!("OUT_DIR"), "/argyle.rs")));

	// See what we've got!
	let mut config = None;
	let mut flags = 0_u8;
	for arg in args {
		match arg {
			Argument::Key("--disable") => { flags |= CLI_DISABLE; },
			Argument::Key("-q" | "--quiet") => { flags |= CLI_QUIET; },
			Argument::Key("--show") => { flags |= CLI_SHOW; },
			Argument::Key("--stdout") => { flags |= CLI_STDOUT; },
			Argument::Key("--systemd") => { flags |= CLI_SYSTEMD; },
			Argument::Key("-y" | "--yes") => { flags |= CLI_YES; },

			Argument::Key("-h" | "--help") => return Err(AdbyssError::PrintHelp),
			Argument::Key("-V" | "--version") => return Err(AdbyssError::PrintVersion),

			Argument::KeyWithValue("-c" | "--config", s) => { config.replace(s); },

			// Nothing else is expected.
			Argument::Other(s) => if s.starts_with('-') {
				return Err(AdbyssError::InvalidCli(s));
			},
			Argument::InvalidUtf8(s) => return Err(AdbyssError::InvalidCli(s.to_string_lossy().into_owned())),
			_ => {},
		}
	}

	// Build the proper settings.
	if config.is_none() && matches!(std::fs::exists(Settings::DEFAULT_CONFIG), Ok(true)) {
		config.replace(Settings::DEFAULT_CONFIG.to_owned());
	}
	let settings =
		if let Some(config) = config { Settings::from_file(config)? }
		else { Settings::default() };

	// Remove everything?
	if CLI_DISABLE == flags & CLI_DISABLE {
		return settings.unwrite(CLI_YES == flags & CLI_YES);
	}

	// Make sure we're online if any sources other than our own are enabled.
	if settings.needs_internet() { check_internet()?; }

	// Just print the domains.
	if CLI_SHOW == flags & CLI_SHOW {
		let shitlist = settings.shitlist()?.into_vec();
		if shitlist.is_empty() { return Err(AdbyssError::NoShitlist); }

		let mut handle = std::io::stdout().lock();
		for v in shitlist { let _res = writeln!(&mut handle, "{v}"); }
		let _res = handle.flush();
	}
	// Build the shitlist, but print it instead of saving it.
	else if CLI_STDOUT == flags & CLI_STDOUT {
		let (out, _) = settings.build()?;
		let mut handle = std::io::stdout().lock();
		let _res = handle.write_all(out.as_bytes()).and_then(|()| handle.flush());
	}
	// Actually write the changes to the host file!
	else {
		let len = settings.write(CLI_YES == flags & CLI_YES)?;

		// Summarize what we've done.
		if CLI_SYSTEMD == flags & CLI_SYSTEMD {
			println!(
				"{} unique hosts have been cast to a blackhole!",
				NiceU64::from(len),
			);
		}
		else if 0 == flags & CLI_QUIET {
			Msg::success(
				format!(
					"{} unique hosts have been cast to a blackhole!",
					NiceU64::from(len),
				)
			).print();
		}
	}

	Ok(())
}

/// # Check Internet.
///
/// This method attempts to check for an internet connection by trying to reach
/// Github (which is serving one of the lists Adbyss needs anyway). It will
/// give it ten tries, with ten seconds in between each try, returning an
/// error if nothing has been reached after that.
///
/// ## Errors
///
/// If the site can't be reached, an error will be returned.
fn check_internet() -> Result<(), AdbyssError> {
	use std::{
		thread::sleep,
		time::Duration,
	};

	let mut tries: u8 = 0;
	loop {
		// Are you there?
		let res = minreq::head("https://github.com/")
			.with_header("user-agent", "Mozilla/5.0")
			.with_timeout(15)
			.send();

		if res.is_ok_and(|r| r.status_code == 200) { return Ok(()); }

		// Out of tries?
		if tries == 9 { return Err(AdbyssError::NoInternet); }

		// Wait and try again.
		tries += 1;
		sleep(Duration::from_secs(10));
	}
}

/// # Require Root.
///
/// This will restart the command with root privileges if necessary, or fail
/// if that doesn't work.
fn require_root() -> Result<(), AdbyssError> {
	use nix::unistd::Uid;

	// We're already root.
	if Uid::effective().is_root() {
		if Uid::current().is_root() { Ok(()) }
		// Almost… we just need to SETUID.
		else {
			nix::unistd::setuid(Uid::from_raw(0)).map_err(|_| AdbyssError::Root)
		}
	}
	// We need to escalate!
	else {
		// Relaunch the command with sudo escalation.
		let mut child = Command::new("/usr/bin/sudo")
			.args(std::env::args())
			.spawn()
			.map_err(|_| AdbyssError::Root)?;

		// Wait to see what happens.
		let exit = child.wait()
			.map_err(|_| AdbyssError::Root)?;

		// Exit this (the old) instance with an appropriate code.
		std::process::exit(
			if exit.success() { 0 }
			else { exit.code().unwrap_or(1) }
		);
	}
}
