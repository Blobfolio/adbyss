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



## License

Copyright © 2020 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004

    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>

    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION

    0. You just DO WHAT THE FUCK YOU WANT TO.

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

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod settings;

use adbyss_core::FLAG_Y;
use fyi_menu::Argue;
use fyi_msg::{
	Msg,
	MsgKind,
	NiceInt,
};
use settings::Settings;
use std::{
	path::PathBuf,
	process::Command,
};



/// Main.
fn main() {
	// We need root!
	if require_root().is_err() {
		MsgKind::Error
			.into_msg("Adbyss requires root privileges.")
			.eprintln();
		std::process::exit(1);
	}

	// Parse CLI arguments.
	let mut args = Argue::new(0)
		.with_version(b"Adbyss", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper);

	// Load configuration. If the user specified one, go with that and print an
	// error if the path is invalid. Otherwise look for a config at the default
	// path and go with that if it exists. Otherwise just use the internal
	// default settings.
	let mut shitlist = args.option2("-c", "--config")
		.map(PathBuf::from)
		.or_else(|| Some(Settings::config()).filter(|x| x.is_file()))
		.map_or_else(
			Settings::default,
			|x|
				if let Ok(y) = std::fs::canonicalize(x) { Settings::from(y) }
				else {
					MsgKind::Error
						.into_msg("Missing configuration.")
						.eprintln();
					std::process::exit(1);
				}
		)
		.into_shitlist();

	// Handle runtime flags.
	if args.switch2("-y", "--yes") {
		shitlist.set_flags(FLAG_Y);
	}

	// Are we just disabling it?
	if args.switch("--disable") {
		if let Err(e) = shitlist.unwrite() {
			MsgKind::Error.into_msg(&e).eprintln();
			std::process::exit(1);
		}
		else { return; }
	}

	// Build it.
	match shitlist.build() {
		Ok(shitlist) =>
			// Just list the results.
			if args.switch("--show") {
				use std::io::Write;

				let raw: String = shitlist.into_vec().join("\n");
				let writer = std::io::stdout();
				let mut handle = writer.lock();
				let _ = handle.write_all(raw.as_bytes())
					.and_then(|_| handle.write_all(b"\n"))
					.and_then(|_| handle.flush());
			}
			// Output to STDOUT?
			else if args.switch("--stdout") {
				use std::io::Write;

				let writer = std::io::stdout();
				let mut handle = writer.lock();
				let _ = handle.write_all(shitlist.as_bytes())
					.and_then(|_| handle.write_all(b"\n"))
					.and_then(|_| handle.flush());
			}
			// Write changes to file.
			else if let Err(e) = shitlist.write() {
				MsgKind::Error.into_msg(&e).eprintln();
				std::process::exit(1);
			}
			// Summarize the results!
			else if ! args.switch2("-q", "--quiet") {
				MsgKind::Success
					.into_msg(&format!(
						"{} unique hosts have been cast to a blackhole!",
						NiceInt::from(shitlist.len()).as_str()
					))
					.println();
			},
		Err(e) => {
			MsgKind::Error.into_msg(&e).eprintln();
			std::process::exit(1);
		}
	}

}

#[cold]
/// Print Help.
fn helper(_: Option<&str>) {
	Msg::from(format!(
		r#"
 .--,       .--,
( (  \.---./  ) )
 '.__/o   o\__.'
    (=  ^  =)       {}{}{}
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

"#,
		"\x1b[38;5;199mAdbyss\x1b[0;38;5;69m v",
		env!("CARGO_PKG_VERSION"),
		"\x1b[0m",
	)).print()
}

#[allow(clippy::result_unit_err)] // We print an error and exit from the caller.
#[allow(clippy::similar_names)] // There's just two variables; we'll be fine.
/// # Require Root.
///
/// This will restart the command with root privileges if necessary, or fail
/// if that doesn't work.
pub fn require_root() -> Result<(), ()> {
	// See what privileges we have.
	let (uid, euid) = unsafe { (libc::getuid(), libc::geteuid()) };

	// We're already root.
	if uid == 0 && euid == 0 { Ok(()) }
	// We just need to SETUID.
	else if euid == 0 {
		unsafe { libc::setuid(0); }
		Ok(())
	}
	// We need to escalate!
	else {
		// Relaunch the command with sudo escalation.
		let mut child = Command::new("/usr/bin/sudo")
			.args(std::env::args())
			.spawn()
			.map_err(|_| ())?;

		// Wait to see what happens.
		let exit = child.wait()
			.map_err(|_| ())?;

		// Exit this (the old) instance with an appropriate code.
		std::process::exit(
			if exit.success() {0}
			else { exit.code().unwrap_or(1) }
		);
	}
}
