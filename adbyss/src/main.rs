/*!
# `Adbyss`
*/

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
	unsafe_code,
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



mod settings;

use adbyss_core::{
	AdbyssError,
	FLAG_Y,
};
use argyle::Argument;
use fyi_msg::Msg;
use dactyl::NiceU64;
use settings::Settings;
use std::{
	path::PathBuf,
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
const CLI_SYSTEMD: u8 = 0b0001_0000;

/// # CLI: Yes to Prompts.
const CLI_YES: u8 =     0b0010_0000;


/// Main.
fn main() {
	match _main() {
		Err(AdbyssError::PrintVersion) => {
			println!(concat!("Adbyss v", env!("CARGO_PKG_VERSION")));
		},
		Err(AdbyssError::PrintHelp) => { helper(); },
		Err(e) => { Msg::error(e.to_string()).die(1); },
		Ok(()) => {},
	}
}

#[inline]
/// Actual Main.
fn _main() -> Result<(), AdbyssError> {
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

			Argument::KeyWithValue("-c" | "--config", s) => {
				config.replace(PathBuf::from(s));
			},

			// Nothing else is expected.
			Argument::Other(s) => if s.starts_with('-') {
				return Err(AdbyssError::InvalidCli(s.into_boxed_str()));
			},
			Argument::InvalidUtf8(s) => return Err(AdbyssError::InvalidCli(s.to_string_lossy().into_owned().into_boxed_str())),
			_ => {},
		}
	}

	// Load configuration. If the user specified one, go with that and print an
	// error if the path is invalid. Otherwise look for a config at the default
	// path and go with that if it exists. Otherwise just use the internal
	// default settings.
	let mut shitlist =
		if let Some(sh) = config
			.or_else(|| Some(Settings::config()).filter(|x| x.is_file()))
		{
			Settings::try_from(sh)?
		}
		else { Settings::default() }
		.into_shitlist();

	// Handle runtime flags.
	let systemd = CLI_SYSTEMD == flags & CLI_SYSTEMD; // A special mode for systemd runs.
	if systemd || CLI_YES == flags & CLI_YES {
		shitlist.set_flags(FLAG_Y);
	}

	// Are we just removing shitlist rules?
	if CLI_DISABLE == flags & CLI_DISABLE { return shitlist.unwrite(); }

	// Make sure we're online if in systemd mode.
	if systemd { adbyss_core::check_internet()?; }

	// Build it.
	let shitlist = shitlist.build()?;

	// Just list the results.
	if CLI_SHOW == flags & CLI_SHOW {
		use std::io::Write;

		let raw: String = shitlist.into_vec().join("\n");
		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _res = handle.write_all(raw.as_bytes())
			.and_then(|()| handle.write_all(b"\n"))
			.and_then(|()| handle.flush());
	}
	// Output to STDOUT? This is like `--show`, but formatted as a hosts file.
	else if CLI_STDOUT == flags & CLI_STDOUT {
		use std::io::Write;

		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _res = handle.write_all(shitlist.as_bytes())
			.and_then(|()| handle.write_all(b"\n"))
			.and_then(|()| handle.flush());
	}
	// Actually write the changes to the host file!
	else {
		shitlist.write()?;

		// Summarize what we've done.
		if systemd {
			println!(
				"{} unique hosts have been cast to a blackhole!",
				NiceU64::from(shitlist.len()).as_str()
			);
		}
		else if 0 == flags & CLI_QUIET {
			Msg::success(
				format!(
					"{} unique hosts have been cast to a blackhole!",
					NiceU64::from(shitlist.len()).as_str()
				)
			).print();
		}
	}

	Ok(())
}

#[cold]
/// Print Help.
fn helper() {
	println!(concat!(
		r#"
 .--,       .--,
( (  \.---./  ) )
 '.__/o   o\__.'
    (=  ^  =)       "#, "\x1b[38;5;199mAdbyss\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r#"
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
    -c, --config <path>    Use this configuration instead of /etc/adbyss.yaml.

SOURCES:
    AdAway:       <https://adaway.org/>
    Steven Black: <https://github.com/StevenBlack/hosts>
    Yoyo:         <https://pgl.yoyo.org/adservers/>

Additional global settings are stored in /etc/adbyss.yaml.
"#
	));
}

#[expect(unsafe_code, reason = "For root.")]
/// # Require Root.
///
/// This will restart the command with root privileges if necessary, or fail
/// if that doesn't work.
fn require_root() -> Result<(), AdbyssError> {
	// Safety: we have to trust libc knows how to fetch the user and group!
	let (uid, e_uid) = unsafe { (libc::getuid(), libc::geteuid()) };

	// We're already root.
	if uid == 0 && e_uid == 0 { Ok(()) }
	// We just need to SETUID.
	else if e_uid == 0 {
		// Safety: again we have to trust this works correctly.
		unsafe { libc::setuid(0); }
		Ok(())
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
