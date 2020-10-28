/*!
# `Adbyss`

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
	set_watermark,
	Shitlist,
};
use fyi_menu::Argue;
use fyi_msg::{
	Msg,
	MsgKind,
	NiceInt,
};
use rayon::prelude::*;
use std::{
	collections::HashSet,
	path::PathBuf,
};



/// Main.
fn main() {
	// Parse CLI arguments.
	let mut args = Argue::new(0)
		.with_version(b"Adbyss", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper);

	// What file are we looking at?
	let input: PathBuf = args.option2("-i", "--input")
		.and_then(|x| std::fs::canonicalize(x).ok())
		.map_or(PathBuf::from("/etc/hosts"), |src| src);

	// Can we read the file?
	let res = read_hosts(&input);
	if let Err(txt) = res {
		MsgKind::Error.into_msg(&txt).eprintln();
		std::process::exit(1);
	}

	// Extract the working hosts.
	let mut hosts = res.unwrap();

	// Add in what we're meant to add in.
	let mut shitlist = fetch_shitlist(args.option("--filter"));

	// Remove bits from the result.
	if let Some(exclude) = args.option("--ignore") {
		exclude.split(',')
			.map(|x| x.trim().to_lowercase())
			.for_each(|x| {
				shitlist.remove(&x);
			});
	}

	// Sort the shitlist and add it to the original.
	let mut shitlist: Vec<String> = shitlist.into_iter().collect();
	shitlist.par_sort();

	shitlist.iter()
		.for_each(|x| {
			hosts.push_str(&format!("\n0.0.0.0 {}", x));
		});

	// Send it to STDOUT.
	if args.switch("--stdout") {
		println!("{}", hosts);
	}
	// Write it somewhere.
	else {
		let output: PathBuf = args.option2("-o", "--output")
			.and_then(|x| std::fs::canonicalize(x).ok())
			.map_or(input, |src| src);

		if write_hosts(&output, &hosts).is_err() {
			MsgKind::Error
				.into_msg(&format!("Unable to write hosts: {:?}", output))
				.eprintln();
			std::process::exit(1);
		}

		MsgKind::Success
			.into_msg(&format!(
				"Blackholed {} hosts in {:?}!",
				NiceInt::from(shitlist.len()).as_str(),
				output
			))
			.println();
	}
}

/// Read Hosts.
fn read_hosts(src: &PathBuf) -> Result<String, String> {
	if ! src.is_file() {
		return Err(format!("Invalid host file: {:?}", src));
	}

	if let Ok(mut res) = std::fs::read_to_string(src) {
		set_watermark(&mut res);
		Ok(res)
	}
	else {
		Err(String::from("Sudo privileges are (probably) required."))
	}
}

/// Fetch Shitlist.
fn fetch_shitlist(src: Option<&str>) -> HashSet<String> {
	match src {
		Some(src) => src.split(',')
			.map(Shitlist::from)
			.collect::<Vec<Shitlist>>(),
		None => vec![
			Shitlist::AdAway,
			Shitlist::Adbyss,
			Shitlist::Marfjeh,
			Shitlist::StevenBlack,
			Shitlist::Yoyo,
		],
	}.par_iter()
		.flat_map(|x| x.fetch())
		.collect()
}

/// Write Hosts.
fn write_hosts(path: &PathBuf, txt: &str) -> Result<(), ()> {
	use std::io::Write;

	let mut temp = tempfile_fast::Sponge::new_for(path).map_err(|_| ())?;
	temp.write_all(txt.as_bytes()).map_err(|_| ())?;
	temp.commit().map_err(|_| ())?;
	Ok(())
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
