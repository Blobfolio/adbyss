#[cfg(not(feature = "man"))]
/// # Do Nothing.
///
/// We only need to rebuild stuff for new releases. The "man" feature is
/// basically used to figure that out.
fn main() {}

#[cfg(feature = "man")]
/// # Build BASH Completions.
///
/// We can do this in the same run we use for building the MAN pages.
fn main() {
	make_bash();
	make_man();
}



#[cfg(feature = "man")]
/// # Build BASH Completions.
fn make_bash() {
	use fyi_menu::Basher;
	use std::{
		env,
		path::PathBuf,
	};

	// We're going to shove this in "adbyss/misc/adbyss.bash". If we used
	// `OUT_DIR` like Cargo suggests, we'd never be able to find it to shove
	// it into the `.deb` package.
	let mut path: PathBuf = env::var("CARGO_MANIFEST_DIR")
		.ok()
		.and_then(|x| std::fs::canonicalize(x).ok())
		.expect("Missing completion script directory.");

	path.push("misc");
	path.push("adbyss.bash");

	// All of our options.
	let b = Basher::new("adbyss")
		.with_option(Some("-c"), Some("--config"))
		.with_switch(None, Some("--no-backup"))
		.with_switch(None, Some("--no-preserve"))
		.with_switch(None, Some("--no-summarize"))
		.with_switch(None, Some("--stdout"))
		.with_switch(Some("-h"), Some("--help"))
		.with_switch(Some("-V"), Some("--version"))
		.with_switch(Some("-y"), Some("--yes"));

	// Write it!
	b.write(&path)
		.unwrap_or_else(|_| panic!("Unable to write completion script: {:?}", path));
}



#[cfg(feature = "man")]
/// # Build MAN Page.
fn make_man() {
	use fyi_menu::{
		Man,
		ManSection,
		ManSectionItem,
	};
	use std::{
		env,
		path::PathBuf,
	};

	// Build the output path.
	let mut path: PathBuf = env::var("CARGO_MANIFEST_DIR")
		.ok()
		.and_then(|x| std::fs::canonicalize(x).ok())
		.expect("Missing completion script directory.");

	path.push("misc");
	path.push("adbyss.1");

	// Build the manual!
	let m = Man::new("Adbyss", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
		.with_text(
			"DESCRIPTION",
			env!("CARGO_PKG_DESCRIPTION"),
			false
		)
		.with_text(
			"USAGE:",
			"adbyss [FLAGS] [OPTIONS]",
			true
		)
		.with_section(
			ManSection::list("FLAGS:")
				.with_item(
					ManSectionItem::new("Print help information.")
						.with_key("-h")
						.with_key("--help")
				)
				.with_item(
					ManSectionItem::new("Do *not* back up the hostfile when writing changes.")
						.with_key("--no-backup")
				)
				.with_item(
					ManSectionItem::new("Do *not* preserve custom entries from the hostfile when writing changes.")
						.with_key("--no-preserve")
				)
				.with_item(
					ManSectionItem::new("Do *not* summarize changes after write.")
						.with_key("--no-summarize")
				)
				.with_item(
					ManSectionItem::new("Print compiled hostfile to STDOUT.")
						.with_key("--stdout")
				)
				.with_item(
					ManSectionItem::new("Print version information.")
						.with_key("-V")
						.with_key("--version")
				)
				.with_item(
					ManSectionItem::new("Non-interactive mode; answer \"yes\" to all prompts.")
						.with_key("-y")
						.with_key("--yes")
				)
		)
		.with_section(
			ManSection::list("OPTIONS:")
				.with_item(
					ManSectionItem::new("Use this configuration instead of /etc/adbyss.yaml.")
						.with_key("-c")
						.with_key("--config")
						.with_value("<FILE>")
				)
		)
		.with_text(
			"GLOBAL:",
			"Additional settings are stored in /etc/adbyss.yaml.",
			true
		)
		.with_section(
			ManSection::list("SOURCE LISTS:")
				.with_item(
					ManSectionItem::new("<https://adaway.org/>")
						.with_value("AdAway")
				)
				.with_item(
					ManSectionItem::new("<https://github.com/StevenBlack/hosts>")
						.with_value("Steven Black")
				)
				.with_item(
					ManSectionItem::new("<https://pgl.yoyo.org/adservers/>")
						.with_value("Yoyo")
				)
		);

	// Write it!
	m.write(&path)
		.unwrap_or_else(|_| panic!("Unable to write MAN script: {:?}", path));
}
