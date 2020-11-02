#[cfg(feature = "man")]
/// # Build BASH Completions.
///
/// We can do this in the same run we use for building the MAN pages.
fn main() {
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

#[cfg(not(feature = "man"))]
/// # Do Nothing.
fn main() {}
