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



## Installation

This application is written in [Rust](https://www.rust-lang.org/) and can be installed using [Cargo](https://github.com/rust-lang/cargo).

For stable Rust (>= `1.47.0`), run:
```bash
RUSTFLAGS="-C link-arg=-s" cargo install \
    --git https://github.com/Blobfolio/adbyss.git \
    --bin adbyss \
    --target x86_64-unknown-linux-gnu
```

Pre-built `.deb` packages are also added for each [release](https://github.com/Blobfolio/adbyss/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy. Just run `sudo adbyss [FLAGS] [OPTIONS]`.

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

And the following options are available:
```bash
--filter <lists>    Specify which of [adaway, adbyss, stevenblack,
                    yoyo] to use, separating multiple lists with
                    commas. [default: all]
--hostfile <path>   Hostfile to use. [default: /etc/hosts]
--exclude <hosts>   Comma-separated list of hosts to *not* blacklist.
--regexclude <pats> Same as --exclude except it takes a comma-separated
                    list of regular expressions.
--include <hosts>   Comma-separated list of additional hosts to
                    blacklist.
```

Click [here](https://docs.rs/regex/1.4.1/regex/index.html#syntax) for regular expression syntax information.

After running Adbyss for the first time, you might find some web sites are no longer working as expected. Most likely you're blocking an evil dependency the web site thinks it *needs*. No worries, just open your browser's Network Dev Tool window and reload the page. Make note of any failing domain(s), and rerun Adbyss with `--exclude domain1,domain2,etc`.

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

use adbyss_core::{
	Shitlist,
	FLAG_ALL,
	FLAG_BACKUP,
	FLAG_FRESH,
	FLAG_SUMMARIZE,
	FLAG_Y,
};
use fyi_menu::Argue;
use fyi_msg::Msg;



/// Main.
fn main() {
	// Parse CLI arguments.
	let mut args = Argue::new(0)
		.with_version(b"Adbyss", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper);

	// Handle flags.
	let mut flags: u8 = FLAG_SUMMARIZE | FLAG_BACKUP;
	if args.switch("--no-backup") { flags &= ! FLAG_BACKUP; }
	if args.switch("--no-preserve") { flags |= FLAG_FRESH; }
	if args.switch("--no-summarize") { flags &= ! FLAG_SUMMARIZE; }
	if args.switch2("-y", "--yes") { flags |= FLAG_Y; }

	let mut shitlist: Shitlist = Shitlist::default()
		.with_flags(flags);

	// Custom hostfile.
	if let Some(h) = args.option("--hostfile") {
		shitlist.set_hostfile(h);
	}

	// Custom excludes.
	if let Some(e) = args.option("--exclude") {
		shitlist.exclude(e.split(',').map(String::from));
	}
	if let Some(e) = args.option("--regexclude") {
		shitlist.regexclude(e.split(',').map(String::from));
	}

	// Custom includes.
	if let Some(i) = args.option("--include") {
		shitlist.include(i.split(',').map(String::from));
	}

	// Custom sources.
	if let Some(s) = args.option("--filter") {
		shitlist.set_sources(s.split(',').map(String::from));
	}
	else {
		shitlist.set_flags(FLAG_ALL);
	}

	// Build it.
	shitlist.build();

	// Output to STDOUT?
	if args.switch("--stdout") {
		println!("{}", shitlist.as_str());
	}
	// Write changes to file.
	else {
		shitlist.write();
	}
}

#[cfg(not(feature = "man"))]
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

{}"#,
		"\x1b[38;5;199mAdbyss\x1b[0;38;5;69m v",
		env!("CARGO_PKG_VERSION"),
		"\x1b[0m",
		include_str!("../misc/help.txt")
	)).print()
}

#[cfg(feature = "man")]
#[cold]
/// Print Help.
///
/// This is a stripped-down version of the help screen made specifically for
/// `help2man`, which gets run during the Debian package release build task.
fn helper(_: Option<&str>) {
	Msg::from([
		b"Adbyss ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n",
		env!("CARGO_PKG_DESCRIPTION").as_bytes(),
		b"\n\n",
		include_bytes!("../misc/help.txt"),
	].concat())
		.print();
}
