/*!
# Adbyss: Public Suffix - Build
*/

use ahash::RandomState;
use regex::Regex;
use std::{
	collections::{
		HashMap,
		HashSet,
	},
	fs::{
		File,
		Metadata,
	},
	io::Write,
	path::PathBuf,
};



// Hash State.
const AHASH_STATE: ahash::RandomState = ahash::RandomState::with_seeds(13, 19, 23, 71);



/// # Build Suffix RS.
///
/// This parses the raw lines of `public_suffix_list.dat` to build out valid
/// Rust code that can be included in `lib.rs`.
///
/// It's a bit ugly, but saves having to do this at runtime!
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=skel/list.rs.txt");

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-list.rs"))
		.expect("Unable to create adbyss-list.rs");

	// Compile the data.
	let (psl_main, psl_wild) = load_data();
	let (main_len, main_inserts) = build_psl_main(psl_main);
	let (wild_len, wild_inserts) = build_psl_wild(psl_wild);

	// Make sure they aren't empty.
	assert!(0 < main_len, "Invalid PSL.");
	assert!(0 < wild_len, "Invalid PSL.");

	write!(
		&mut file,
		include_str!("./skel/list.rs.txt"),
		main_len = main_len,
		main_inserts = main_inserts,
		wild_len = wild_len,
		wild_inserts = wild_inserts
	)
		.and_then(|_| file.flush())
		.unwrap();
}

#[cfg(not(feature = "docs-workaround"))]
/// # Load Data.
fn load_data() -> (HashSet<String, RandomState>, HashMap<String, Vec<String>, RandomState>) {
	// Let's build the thing we'll be writing about building.
	let mut psl_main: HashSet<String, RandomState> = HashSet::with_hasher(AHASH_STATE);
	let mut psl_wild: HashMap<String, Vec<String>, RandomState> = HashMap::with_hasher(AHASH_STATE);

	const FLAG_EXCEPTION: u8 = 0b0001;
	const FLAG_WILDCARD: u8  = 0b0010;

	// Parse the raw data.
	fetch_suffixes().lines()
		.filter(|line| ! line.is_empty() && ! line.starts_with("//"))
		.filter_map(|mut line| {
			let mut flags: u8 = 0;

			if line.starts_with('!') {
				line = &line[1..];
				flags |= FLAG_EXCEPTION;
			}

			if line.starts_with("*.") {
				line = &line[2..];
				flags |= FLAG_WILDCARD;
			}

			// To correctly handle the suffixes, we'll need to prepend a
			// hypothetical root and strip it off after.
			idna::domain_to_ascii_strict(&["one.two.", line].concat())
				.ok()
				.map(|mut x| x.split_off(8))
				.zip(Some(flags))
		})
		.for_each(|(host, flag)|
			// This is a wildcard exception.
			if 0 != flag & FLAG_EXCEPTION {
				if let Some(idx) = host.as_bytes()
					.iter()
					.position(|x| x == &b'.')
				{
					let (before, after) = host.split_at(idx);
					psl_wild.entry(after[1..].to_string())
						.or_insert_with(Vec::new)
						.push(before.to_string());
				}
			}
			// This is the main wildcard entry.
			else if 0 != flag & FLAG_WILDCARD {
				psl_wild.entry(host)
					.or_insert_with(Vec::new);
			}
			// This is a normal suffix.
			else {
				psl_main.insert(host);
			}
		);

	(psl_main, psl_wild)
}

#[cfg(feature = "docs-workaround")]
/// # (Fake) Load Data.
///
/// This is a network-free workaround to allow `docs.rs` to be able to generate
/// documentation for this library.
///
/// Don't try to compile this library with the `docs-workaround` feature or the
/// library won't work properly.
fn load_data() -> (HashSet<String, RandomState>, HashMap<String, Vec<String>, RandomState>) {
	let mut psl_main: HashSet<String, RandomState> = HashSet::with_hasher(AHASH_STATE);
	psl_main.insert(String::from("com"));

	let mut psl_wild: HashMap<String, Vec<String>, RandomState> = HashMap::with_hasher(AHASH_STATE);
	psl_wild.insert(String::from("bd"), Vec::new());

	(psl_main, psl_wild)
}

/// # Build PSL_MAIN.
fn build_psl_main(set: HashSet<String, RandomState>) -> (usize, String) {
	let mut set: Vec<String> = set.iter()
		.map(|x| format!("\tout.insert(\"{}\");\n", x))
		.collect();
	set.sort();

	(
		set.len(),
		set.concat(),
	)
}

/// # Build PSL_WILD.
fn build_psl_wild(set: HashMap<String, Vec<String>, RandomState>) -> (usize, String) {
	let mut set: Vec<String> = set.iter()
		.map(|(k, v)| format!(
			"\tout.insert(\"{}\", vec![{}]);\n",
			k,
			v.iter()
				.map(|x| format!(r#""{}""#, x))
				.collect::<Vec<String>>()
				.join(", ")
		))
		.collect();
	set.sort();

	(
		set.len(),
		set.concat(),
	)
}

#[cfg(not(feature = "docs-workaround"))]
/// # Fetch Suffixes.
///
/// This downloads and lightly cleans the raw public suffix list.
fn fetch_suffixes() -> String {
	// Cache this locally for up to an hour.
	let cache = out_path("public_suffix_list.dat");
	if let Some(x) = std::fs::metadata(&cache)
		.ok()
		.filter(Metadata::is_file)
		.and_then(|meta| meta.modified().ok())
		.and_then(|time| time.elapsed().ok().filter(|secs| secs.as_secs() < 3600))
		.and_then(|_| std::fs::read_to_string(&cache).ok())
	{
		return x;
	}

	// Download it fresh.
	let raw = ureq::get("https://publicsuffix.org/list/public_suffix_list.dat")
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(|e| e.into()))
		.map(|raw| {
			let re = Regex::new(r"(?m)^\s*").unwrap();
			re.replace_all(&raw, "").to_string()
		})
		.expect("Unable to fetch https://publicsuffix.org/list/public_suffix_list.dat");

	// We don't need to panic if the cache-save fails; the system is only
	// hurting its future self in such cases. ;)
	let _res = File::create(cache)
		.and_then(|mut file| file.write_all(raw.as_bytes()).and_then(|_| file.flush()));

	// Return the data.
	raw
}

/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	let mut out = std::fs::canonicalize(dir).expect("Missing OUT_DIR.");
	out.push(name);
	out
}
