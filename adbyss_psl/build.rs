/*!
# Adbyss: Public Suffix - Build
*/

use ahash::RandomState;
use dactyl::NiceU64;
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

type RawMainMap = HashSet<String, RandomState>;
type RawWildMap = HashMap<String, Vec<String>, RandomState>;
type HostHashes = HashMap<String, u64, RandomState>;



/// # Build Suffix RS.
///
/// This parses the raw lines of `public_suffix_list.dat` to build out valid
/// Rust code that can be included in `lib.rs`.
///
/// It's a bit ugly, but saves having to do this at runtime!
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=skel/list.rs.txt");

	// Pull the raw data.
	let (psl_main, psl_wild) = load_data();
	assert!(! psl_main.is_empty(), "No generic PSL entries found.");
	assert!(! psl_wild.is_empty(), "No wildcard PSL entries found.");

	// Reformat it.
	let (psl_kinds, psl_kind_arms, len, inserts) = build_data(psl_main, psl_wild);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-list.rs"))
		.expect("Unable to create adbyss-list.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/list.rs.txt"),
		psl_kinds = psl_kinds,
		psl_kind_arms = psl_kind_arms,
		len = len,
		inserts = inserts,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
}

#[cfg(not(feature = "docs-workaround"))]
/// # Load Data.
fn load_data() -> (RawMainMap, RawWildMap) {
	// Let's build the thing we'll be writing about building.
	let mut psl_main: RawMainMap = HashSet::with_hasher(AHASH_STATE);
	let mut psl_wild: RawWildMap = HashMap::with_hasher(AHASH_STATE);

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
fn load_data() -> (RawMainMap, RawWildMap) {
	let mut psl_main: RawMainMap = HashSet::with_hasher(AHASH_STATE);
	psl_main.insert(String::from("com"));

	let mut psl_wild: RawWildMap = HashMap::with_hasher(AHASH_STATE);
	psl_wild.insert(String::from("bd"), Vec::new());

	(psl_main, psl_wild)
}

/// # Build Data.
fn build_data(main: RawMainMap, wild: RawWildMap) -> (String, String, usize, String) {
	let hashes: HostHashes = host_hashes(&main, &wild);
	let mut all: HashMap<u64, String, RandomState> = HashMap::with_capacity_and_hasher(hashes.len(), AHASH_STATE);
	let mut kinds: HashMap<String, String, RandomState> = HashMap::with_hasher(AHASH_STATE);
	let mut kind_arms: Vec<(String, String)> = Vec::new();

	// Suck up the main rules first; they're easy.
	all.extend(
		main.into_iter()
			.map(|x| {
				let hash = hashes.get(&x).unwrap();
				(*hash, String::from("Normal"))
			})
	);

	// The wild entries take a little more effort.
	all.extend(
		wild.into_iter()
			.map(|(x, ex)| {
				let hash = hashes.get(&x).unwrap();
				if ex.is_empty() {
					(*hash, String::from("Wild"))
				}
				else {
					let ex: String = format_wild_arm(&ex);
					if ! kinds.contains_key(&ex) {
						let kind = format_wild_kind(&ex);
						let arm =
							if ex.contains('|') {
								format!("matches!(src, {})", ex)
							}
							else {
								format!("src == {}", ex)
							};
						kinds.insert(ex.clone(), kind.clone());
						kind_arms.push((kind, arm));
					}
					let kind = kinds.get(&ex).unwrap();
					(*hash, kind.clone())
				}
			})
	);

	// Let's convert the entries into a vector so we can sort them.
	let len: usize = all.len();
	let mut all: Vec<(u64, String)> = all.into_iter().collect();
	all.sort_by(|a, b| a.0.cmp(&b.0));
	let all: String = all.as_slice().chunks(128)
		.map(|chunk| {
			let list: Vec<String> = chunk.iter()
				.map(|(hash, kind)| format!(
					"({}_u64, Psl::{}), ",
					NiceU64::from(*hash).as_str().replace(",", "_"),
					kind
				))
				.collect();
			format!("\t\t{}", list.concat())
		})
		.collect::<Vec<String>>()
		.join("\n");

	// The kinds, likewise, can be collapsed and reformatted.
	let mut kinds: Vec<String> = kinds.values()
		.map(|v| format!("\t{},", v))
		.collect();
	kinds.sort_unstable();
	let kinds: String = kinds.join("\n");

	// The kind arms can also be formatted, etc.
	kind_arms.sort_by(|a, b| a.0.cmp(&b.0));
	let kind_arms: String = kind_arms.into_iter()
		.map(|(kind, cond)| format!("\t\t\tSelf::{} => {},", kind, cond))
		.collect::<Vec<String>>()
		.join("\n");

	(kinds, kind_arms, len, all)
}

/// # Host Hashes.
fn host_hashes(main: &RawMainMap, wild: &RawWildMap) -> HostHashes {
	let mut out: HostHashes = HashMap::with_capacity_and_hasher(
		main.len() + wild.len(),
		AHASH_STATE,
	);

	out.extend(main.iter().map(|x| (x.to_string(), quick_hash(x.as_bytes()))));
	out.extend(wild.keys().map(|x| (x.to_string(), quick_hash(x.as_bytes()))));
	out
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

/// # Format Exception Match Conditions.
fn format_wild_arm(src: &[String]) -> String {
	let mut out: Vec<String> = src.iter()
		.map(|x| format!(r#"b"{}""#, x))
		.collect();
	out.sort();
	out.join(" | ")
}

/// # Format Exception Name.
fn format_wild_kind(src: &str) -> String {
	let hash = quick_hash(src.as_bytes());
	format!("Wild{}", hash)
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

/// # Path Hash.
///
/// This hashes a device and inode to produce a more or less unique result.
/// This is the value we grab for each path and use in the `HashSet`.
fn quick_hash(raw: &[u8]) -> u64 {
	use std::hash::Hasher;
	let mut hasher = ahash::AHasher::new_with_keys(1319, 2371);
	hasher.write(raw);
	hasher.finish()
}
