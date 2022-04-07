/*!
# `Adbyss`

Adbyss is a DNS blocklist manager for x86-64 Linux machines.

While ad-blocking browser extensions are extremely useful, they only block
unwatned content *in the browser*, and require read/write access to every
page you visit, which adds overhead and potential security/privacy issues.

Adbyss instead writes "blackhole" records directly to your system's `/etc/hosts`
file, preventing all spammy connection attempts system-wide. As this is just a
text file, no special runtime scripts are required, and there is very little
overhead.

**This software is a work-in-progress.**

Feel free to use it, but if something weird happens — or if you have ideas for improvement — please open an [issue](https://github.com/Blobfolio/adbyss/issues)!



## Installation

This application is written in [Rust](https://www.rust-lang.org/) and can be built using [Cargo](https://github.com/rust-lang/cargo). If building manually, don't forget to copy the configuration file:
```bash
sudo cp adbyss/misc/adbyss.yaml /etc
```

Pre-built `.deb` packages are also added for each [release](https://github.com/Blobfolio/adbyss/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy.

Settings are stored in `/etc/adbyss.yaml`. Edit those as needed.

Otherwise, just run `sudo adbyss [FLAGS] [OPTIONS]`.

The following flags are available:
```bash
    --disable       Remove *all* Adbyss entries from the hostfile.
-h, --help          Prints help information.
-q, --quiet         Do *not* summarize changes after write.
    --show          Print a sorted blackholable hosts list to STDOUT, one per
                    line.
    --stdout        Print the would-be hostfile to STDOUT instead of writing
                    it to disk.
-V, --version       Prints version information.
-y, --yes           Non-interactive mode; answer "yes" to all prompts.
```

And the following option is available:
```bash
-c, --config <path> Use this configuration instead of /etc/adbyss.yaml.
```

After running Adbyss for the first time, you might find some web sites are no longer working as expected. Most likely you're blocking an evil dependency the web site thinks it *needs*. No worries, just open your browser's Network Dev Tool window and reload the page. Make note of any failing domain(s), and update the `/etc/adbyss.yaml` configuration accordingly.

Restart your browser and/or computer and everything should be peachy again.

If ads persist in displaying even after running Adbyss and rebooting, double-check the browser isn't bypassing your computer's local DNS records. (Firefox's DNS-Over-HTTPS feature sometimes does this.) Tweak your settings as needed and you should be back in business.

It is important to remember that scammers and capitalists birth new schemes all the time. It is a good idea to rerun Adbyss weekly or so to ensure your hosts list contains the latest updates.



## Removal

To remove all Adbyss rules from your hosts file, either run `adbyss --disable`, or open the hostfile in a text editor, find the big-obvious `# ADBYSS #` marker, and delete it and all subsequent lines.

Save, reboot, and you're back to normal.
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::module_name_repetitions)]



mod settings;

use adbyss_core::{
	AdbyssError,
	FLAG_Y,
};
use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_VERSION,
};
use fyi_msg::Msg;
use dactyl::NiceU64;
use settings::Settings;
use std::{
	path::PathBuf,
	process::Command,
};



/// Main.
fn main() {
	match _main() {
		Err(AdbyssError::Argue(ArgyleError::WantsVersion)) => {
			println!(concat!("Adbyss v", env!("CARGO_PKG_VERSION")));
		},
		Err(AdbyssError::Argue(ArgyleError::WantsHelp)) => {
			helper();
		},
		Err(e) => {
			Msg::error(e.to_string()).die(1);
		},
		Ok(_) => {},
	}
}

#[inline]
/// Actual Main.
fn _main() -> Result<(), AdbyssError> {
	// We need root!
	require_root()?;

	// Parse CLI arguments.
	let args = Argue::new(FLAG_VERSION | FLAG_HELP)?;

	// Load configuration. If the user specified one, go with that and print an
	// error if the path is invalid. Otherwise look for a config at the default
	// path and go with that if it exists. Otherwise just use the internal
	// default settings.
	let mut shitlist =
		if let Some(sh) = args.option2_os(b"-c", b"--config")
			.map(PathBuf::from)
			.or_else(|| Some(Settings::config()).filter(|x| x.is_file()))
		{
			Settings::try_from(sh)?
		}
		else { Settings::default() }
		.into_shitlist();

	// Handle runtime flags.
	let systemd = args.switch(b"--systemd"); // A special mode for systemd runs.
	if systemd || args.switch2(b"-y", b"--yes") {
		shitlist.set_flags(FLAG_Y);
	}

	// Are we just removing shitlist rules?
	if args.switch(b"--disable") {
		return shitlist.unwrite();
	}

	// Make sure we're online if in systemd mode.
	if systemd { adbyss_core::check_internet()?; }

	// Build it.
	let shitlist = shitlist.build()?;

	// Just list the results.
	if args.switch(b"--show") {
		use std::io::Write;

		let raw: String = shitlist.into_vec().join("\n");
		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _res = handle.write_all(raw.as_bytes())
			.and_then(|_| handle.write_all(b"\n"))
			.and_then(|_| handle.flush());
	}
	// Output to STDOUT? This is like `--show`, but formatted as a hosts file.
	else if args.switch(b"--stdout") {
		use std::io::Write;

		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _res = handle.write_all(shitlist.as_bytes())
			.and_then(|_| handle.write_all(b"\n"))
			.and_then(|_| handle.flush());
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
		else if ! args.switch2(b"-q", b"--quiet") {
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

/// # Require Root.
///
/// This will restart the command with root privileges if necessary, or fail
/// if that doesn't work.
fn require_root() -> Result<(), AdbyssError> {
	// See what privileges we have.
	let (uid, e_uid) = unsafe { (libc::getuid(), libc::geteuid()) };

	// We're already root.
	if uid == 0 && e_uid == 0 { Ok(()) }
	// We just need to SETUID.
	else if e_uid == 0 {
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
