/*!
# Adbyss: Public Suffix - Build
*/

use regex::Regex;
use std::{
	collections::{
		HashMap,
		HashSet,
	},
	env,
	fs::File,
	io::Write,
	path::PathBuf,
};



type RawMainMap = HashSet<String>;
type RawWildMap = HashMap<String, Vec<String>>;



/// # Build Resources!
///
/// Apologies for such a massive build script, but the more crunching we can do
/// at build time, the faster the runtime experience will be.
///
/// This method triggers the building of three components:
/// * Public Suffix List;
/// * IDNA/Unicode Tables;
/// * IDNA/Unicode unit tests;
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=skel/psl.rs.txt");
	println!("cargo:rerun-if-changed=skel/raw/public_suffix_list.dat");

	// Sanity check. Obviously this won't change, but it is nice to know we
	// thought of it.
	assert_eq!(
		u64::MAX.to_string().len(),
		RANGE.len(),
		"Number-formatting string has the wrong length.",
	);

	psl();
}

/// # Build Suffix RS.
///
/// This method handles all operations related to the Public Suffix List data.
/// Ultimately, it collects a bunch of Rust "code" represented as strings, and
/// writes them to a pre-formatted template. The generated script is then
/// included by the library.
fn psl() {
	// Pull the raw data.
	let (psl_main, psl_wild) = psl_load_data();
	assert!(! psl_main.is_empty(), "No generic PSL entries found.");
	assert!(! psl_wild.is_empty(), "No wildcard PSL entries found.");
	for host in psl_wild.keys() {
		assert!(! psl_main.contains(host), "Duplicate host.");
	}

	if env::var("SHOW_TOTALS").is_ok() {
		println!(
			"cargo:warning=Parsed {} generic PSL entries, and {} wildcard ones.",
			psl_main.len(),
			psl_wild.len(),
		);
	}

	// Reformat it.
	let (
		map,
		wild_kinds,
		wild_arms,
	) = psl_build_list(&psl_main, &psl_wild);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-psl.rs"))
		.expect("Unable to create adbyss-psl.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/psl.rs.txt"),
		map = map,
		wild_kinds = wild_kinds,
		wild_arms = wild_arms,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
}

/// # Build List.
///
/// This method crunches the (pre-filtered) Public Suffix data into a static
/// hash map we can query at runtime.
///
/// Ultimately, there are three kinds of entries:
/// * TLD: a normal TLD.
/// * Wild: a TLD that comprises both the explicit entry, as well as any arbitrary "subdomain".
/// * Wild-But: a Wild entry that contains one or more exceptions to chunks that may precede it.
fn psl_build_list(main: &RawMainMap, wild: &RawWildMap) -> (String, String, String) {
	// The wild stuff is the hardest.
	let (wild_map, wild_kinds, wild_arms) = psl_build_wild(wild);

	// Hold map key/value pairs.
	let mut map: Vec<(u64, String)> = Vec::with_capacity(main.len() + wild.len());

	// Populate this with stringified tuples (bytes=>kind).
	for host in main {
		// We'll prioritize these.
		if host == "com" || host == "net" || host == "org" { continue; }
		let hash = hash_tld(host.as_bytes());
		map.push((hash, "SuffixKind::Tld".to_owned()));
	}
	for (host, ex) in wild {
		let hash = hash_tld(host.as_bytes());
		if ex.is_empty() {
			map.push((hash, "SuffixKind::Wild".to_owned()));
		}
		else {
			let ex = psl_format_wild(ex);
			let ex = wild_map.get(&ex).expect("Missing wild arm.");
			map.push((hash, format!("SuffixKind::WildEx(WildKind::{ex})")));
		}
	}

	// Make sure the keys are unique.
	{
		let tmp: HashSet<u64> = map.iter().map(|(k, _)| *k).collect();
		assert_eq!(tmp.len(), map.len(), "Duplicate PSL hash keys.");
	}

	let len: usize = map.len();
	map.sort_by(|a, b| a.0.cmp(&b.0));

	// Separate keys and values.
	let (map_keys, map_values): (Vec<u64>, Vec<String>) = map.into_iter().unzip();

	// Format the arrays.
	let map = format!(
		"/// # Map Keys.\nstatic MAP_K: [u64; {len}] = [{}];\n\n/// # Map Values.\nstatic MAP_V: [SuffixKind; {len}] = [{}];",
		map_keys.into_iter()
			.map(nice_u64)
			.collect::<Vec<String>>()
			.join(", "),
		map_values.join(", "),
	);

	(map, wild_kinds, wild_arms)
}

/// # Build Wild Enum.
///
/// There aren't very many wildcard exceptions, so we end up storing them as a
/// static enum at runtime. A matcher function is generated with the
/// appropriate branch tests, which will either be a straight slice comparison
/// or a `[].contains`-type match.
fn psl_build_wild(wild: &RawWildMap) -> (HashMap<String, String>, String, String) {
	// Let's start with the wild kinds and wild arms.
	let mut tmp: Vec<String> = wild.values()
		.filter_map(|ex|
			if ex.is_empty() { None }
			else { Some(psl_format_wild(ex)) }
		)
		.collect();
	tmp.sort();
	tmp.dedup();

	let mut wild_kinds: Vec<String> = Vec::new();
	let mut wild_map: HashMap<String, String> = HashMap::new();
	for (k, v) in tmp.into_iter().enumerate() {
		let name = format!("Ex{k}");
		wild_kinds.push(format!("{name},"));
		wild_map.insert(v, name);
	}

	// If there aren't any wild exceptions, we can just return an empty
	// placeholder that will never be referenced.
	if wild_kinds.is_empty() {
		return (wild_map, "\tNone,".to_owned(), "\t\t\tSelf::None => false,".to_owned());
	}

	let wild_kinds: String = wild_kinds.join("\n");
	let mut wild_arms: Vec<(&String, &String)> = wild_map.iter().collect();
	wild_arms.sort_by(|a, b| a.1.cmp(b.1));
	let wild_arms = wild_arms.into_iter()
		.map(|(cond, name)| format!("\t\t\tSelf::{name} => {cond},"))
		.collect::<Vec<String>>()
		.join("\n");

	(wild_map, wild_kinds, wild_arms)
}

/// # Fetch Suffixes.
///
/// This loads and lightly cleans the raw Public Suffix List data.
fn psl_fetch_suffixes() -> String {
	let raw = load_file("public_suffix_list.dat");
	let re = Regex::new(r"(?m)^\s*").unwrap();
	re.replace_all(&raw, "").to_string()
}

/// # Format Wild Exceptions.
///
/// This builds the match condition for a wildcard exception, which will either
/// take the form of a straight slice comparison, or a `[].contains` match.
///
/// Not all wildcards have exceptions. Just in case, this method will return an
/// empty string in such cases, but those will get filtered out during
/// processing.
fn psl_format_wild(src: &[String]) -> String {
	if src.is_empty() { String::new() }
	else if src.len() == 1 {
		format!("src == b\"{}\"", src[0])
	}
	else {
		format!(
			"[{}].contains(src)",
			src.iter()
				.map(|s| format!("b\"{s}\""))
				.collect::<Vec<String>>()
				.join(", ")
		)
	}
}

/// # Load Data.
///
/// This loads the raw Public Suffix List data, and splits it into two parts:
/// normal and wildcard.
///
/// As with all other "load" methods, this will either download the raw data
/// fresh from `publicsuffix.org`, or when building for `docs.rs` — which
/// doesn't support network actions — pull a stale copy included with this
/// library.
fn psl_load_data() -> (RawMainMap, RawWildMap) {
	// Let's build the thing we'll be writing about building.
	let mut psl_main: RawMainMap = HashSet::with_capacity(9500);
	let mut psl_wild: RawWildMap = HashMap::with_capacity(128);

	const FLAG_EXCEPTION: u8 = 0b0001;
	const FLAG_WILDCARD: u8  = 0b0010;

	// Parse the raw data.
	psl_fetch_suffixes().lines()
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
				if let Some(idx) = host.bytes().position(|x| x == b'.') {
					let (before, after) = host.split_at(idx);
					psl_wild.entry(after[1..].to_string())
						.or_default()
						.push(before.to_string());
				}
			}
			// This is the main wildcard entry.
			else if 0 != flag & FLAG_WILDCARD {
				psl_wild.entry(host).or_default();
			}
			// This is a normal suffix.
			else {
				psl_main.insert(host);
			}
		);

	(psl_main, psl_wild)
}



/// # Load File.
///
/// Read the third-party data file into a string.
fn load_file(name: &str) -> String {
	match std::fs::read_to_string(format!("./skel/raw/{name}")) {
		Ok(x) => x,
		Err(_) => panic!("Unable to load {name}."),
	}
}

/// # Hash TLD.
///
/// This is just a simple wrapper to convert a slice into a u64, used by the
/// suffix map builder.
///
/// In testing, the `ahash` algorithm is far and away the fastest, so that is
/// what we use, both during build and at runtime (i.e. search needles) during
/// lookup matching.
fn hash_tld(src: &[u8]) -> u64 {
	ahash::RandomState::with_seeds(13, 19, 23, 71).hash_one(src)
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

/// # Range covering the length of u64::MAX, stringified.
const RANGE: [usize; 20] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19];

/// # Nice Number.
///
/// This stringifies and returns a number with `_` separators, useful for
/// angering clippy with the code we're generating.
fn nice_u64(num: u64) -> String {
	let digits = num.to_string();

	// Return it straight if no separator is needed.
	let len = digits.len();
	if len < 4 { return digits; }

	// Otherwise split it into chunks of three, starting at the end.
	let mut parts: Vec<&str> = RANGE[..len].rchunks(3)
		.map(|chunk| {
			let (min, chunk) = chunk.split_first().unwrap();
			let max = chunk.split_last().map_or(min, |(max, _)| max);
			&digits[*min..=*max]
		})
		.collect();

	// (Re)reverse and glue with the separator.
	parts.reverse();
	parts.join("_")
}
