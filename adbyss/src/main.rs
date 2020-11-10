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
-h, --help          Prints help information.
    --no-backup     Do *not* back up the hostfile when writing changes.
    --no-preserve   Do *not* preserve custom entries from hostfile when
                    writing changes.
    --no-summarize  Do *not* summarize changes after write.
    --stdout        Send compiled hostfile to STDOUT.
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

To remove all Adbyss rules from your hosts file, simply open the hosts file in a text editor, find the big-obvious `# ADBYSS #` marker, and delete it and everything following it. Save, reboot, and you're back to normal.



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
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod settings;

use adbyss_core::{
	FLAG_BACKUP,
	FLAG_SKIP_HOSTS,
	FLAG_Y,
};
use settings::Settings;
use std::path::PathBuf;
use fyi_menu::Argue;
use fyi_msg::{
	Msg,
	MsgKind,
	NiceInt,
};



/// Main.
fn main() {
	// We need root!
	if sudo::escalate_if_needed().is_err() {
		MsgKind::Error
			.into_msg("Adbyss requires root privileges.")
			.eprintln();
		std::process::exit(1);
	}

	// Parse CLI arguments.
	let mut args = Argue::new(0)
		.with_version(b"Adbyss", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper);

	// Load configuration.
	let mut shitlist = args.option2("-c", "--config")
		.map(PathBuf::from)
		.or_else(|| Some(Settings::config()).filter(|x| x.is_file()))
		.map_or(
			Settings::default(),
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
	if args.switch("--no-backup") {
		shitlist.disable_flags(FLAG_BACKUP);
	}
	if args.switch("--no-preserve") {
		shitlist.set_flags(FLAG_SKIP_HOSTS);
	}
	if args.switch2("-y", "--yes") {
		shitlist.set_flags(FLAG_Y);
	}

	// Build it.
	match shitlist.build() {
		Ok(shitlist) =>
			// Output to STDOUT?
			if args.switch("--stdout") {
				println!("{}", shitlist.as_str());
			}
			// Write changes to file.
			else if let Err(e) = shitlist.write() {
				MsgKind::Error.into_msg(&e).eprintln();
				std::process::exit(1);
			}
			// Summarize the results!
			else if ! args.switch("--no-summarize") {
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
    -h, --help         Prints help information.
        --no-backup    Do *not* back up the hostfile when writing changes.
        --no-preserve  Do *not* preserve custom entries from hostfile when
                       writing changes.
        --no-summarize Do *not* summarize changes after write.
        --stdout       Print compiled hostfile to STDOUT.
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
