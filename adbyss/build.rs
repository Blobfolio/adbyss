#[cfg(not(feature = "man"))]
/// # Do Nothing.
///
/// We only need to rebuild stuff for new releases. The "man" feature is
/// basically used to figure that out.
fn main() {}



#[cfg(feature = "man")]
/// # Build.
fn main() {
	use fyi_menu::{
		Agree,
		AgreeSection,
		AgreeKind,
	};
	use std::{
		env,
		path::PathBuf,
	};

	let app: Agree = Agree::new(
		"Adbyss",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION"),
		env!("CARGO_PKG_DESCRIPTION"),
	)
		.with_arg(
			AgreeKind::switch("Print help information.")
				.with_short("-h")
				.with_long("--help")
		)
		.with_arg(
			AgreeKind::switch("Do *not* back up the hostfile when writing changes.")
				.with_long("--no-backup")
		)
		.with_arg(
			AgreeKind::switch("Do *not* preserve custom entries from the hostfile when writing changes.")
				.with_long("--no-preserve")
		)
		.with_arg(
			AgreeKind::switch("Do *not* summarize changes after write.")
				.with_long("--no-summarize")
		)
		.with_arg(
			AgreeKind::switch("Print compiled hostfile to STDOUT.")
				.with_long("--stdout")
		)
		.with_arg(
			AgreeKind::switch("Print program version.")
				.with_short("-V")
				.with_long("--version")
		)
		.with_arg(
			AgreeKind::switch("Non-interactive mode; answer \"yes\" to all prompts.")
				.with_short("-y")
				.with_long("--yes")
		)
		.with_arg(
			AgreeKind::option("<FILE>", "Use this configuration instead of /etc/adbyss.yaml.", true)
				.with_short("-c")
				.with_long("--config")
		)
		.with_section(
			AgreeSection::new("GLOBAL:", true)
				.with_item(
					AgreeKind::paragraph("Additional settings are stored in /etc/adbyss.yaml. Edit those to set your preferred global runtime behaviors.")
				)
		)
		.with_section(
			AgreeSection::new("SOURCE LISTS:", true)
				.with_item(AgreeKind::item("AdAway", "<https://adaway.org/>"))
				.with_item(AgreeKind::item("Steven Black", "<https://github.com/StevenBlack/hosts>"))
				.with_item(AgreeKind::item("Yoyo", "<https://pgl.yoyo.org/adservers/>"))
		);

	// Our files will go to ./misc.
	let mut path: PathBuf = env::var("CARGO_MANIFEST_DIR")
		.ok()
		.and_then(|x| std::fs::canonicalize(x).ok())
		.expect("Missing output directory.");

	path.push("misc");

	// Write 'em!
	app.write_bash(&path)
		.unwrap_or_else(|_| panic!("Unable to write BASH completion script: {:?}", path));
	app.write_man(&path)
		.unwrap_or_else(|_| panic!("Unable to write MAN page: {:?}", path));
}
