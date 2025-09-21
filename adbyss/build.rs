/*!
# Adbyss: Build
*/

/// # Set Up CLI Arguments and Flags.
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	argyle::FlagsBuilder::new("Flags")
		.private()
		.with_flag("Disable", Some("# Disable Shitlist."))
		.with_flag("Quiet", None)
		.with_flag("Show", Some("# Show Only."))
		.with_flag("Stdout", Some("# Print to STDOUT."))
		.with_complex_flag("Systemd", ["Yes"], Some("# Systemd Use.\n\nImplies `--yes`."))
		.with_flag("Yes", Some("# Assume Yes (Don't Prompt)."))
		.save(out_path("flags.rs"));
}

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> std::path::PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}
