/*!
# Adbyss: Build
*/

use argyle::KeyWordsBuilder;
use std::path::PathBuf;



/// # Set Up CLI Arguments.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"--disable",
		"-h", "--help",
		"-q", "--quiet",
		"--show",
		"--stdout",
		"--systemd",
		"-V", "--version",
		"-y", "--yes",
	]);
	builder.push_keys_with_values(["-c", "--config"]);
	builder.save(out_path("argyle.rs"));
}

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}
