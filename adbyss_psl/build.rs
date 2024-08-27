/*!
# Adbyss: Public Suffix - Build
*/

use regex::Regex;
use std::{
	borrow::Cow,
	collections::{
		BTreeMap,
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

	// We assume com/net/org are normal; let's verify that!
	for i in ["com", "net", "org"] {
		assert!(main.contains(i), "Normal tld list missing {i}!");
		assert!(! wild.contains_key(i), "Wild list contains {i}!");
	}

	// Combine the main and wild data into a single, deduped map, sorted for
	// binary search compatibility, which is how lookups will end up working on
	// the runtime side of the equation.
	let map: BTreeMap<u64, Cow<str>> = main.iter()
		.filter_map(|host|
			// We handle these three common cases manually for performance
			// reasons.
			if host == "com" || host == "net" || host == "org" { None }
			else {
				Some((hash_tld(host.as_bytes()), Cow::Borrowed("SuffixKind::Tld")))
			}
		)
		.chain(
			wild.iter().map(|(host, ex)| {
				let hash = hash_tld(host.as_bytes());
				if ex.is_empty() {
					(hash, Cow::Borrowed("SuffixKind::Wild"))
				}
				else {
					let ex = psl_format_wild(ex);
					let ex = wild_map.get(&ex).expect("Missing wild arm.");
					(hash, Cow::Owned(format!("SuffixKind::WildEx(WildKind::{ex})")))
				}
			})
		)
		.collect();

	// Double-check the lengths; if there's a mismatch we found an (improbable)
	// hash collision and need to fix it.
	let len: usize = map.len();
	assert_eq!(len, main.len() + wild.len() - 3, "Duplicate PSL hash keys!");

	// Separate keys and values.
	let (map_keys, map_values): (Vec<u64>, Vec<Cow<str>>) = map.into_iter().unzip();

	// Format the arrays.
	let map = format!(
		r#"/// # Map Keys.
const MAP_K: &[u64; {len}] = &[{}];

/// # Map Values.
const MAP_V: &[SuffixKind; {len}] = &[{}];
"#,
		nice_map_keys(map_keys),
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
	const FLAG_EXCEPTION: u8 = 0b0001;
	const FLAG_WILDCARD: u8  = 0b0010;
	const STUB: &str = "a.a.";

	/// # Domain to ASCII.
	fn idna_to_ascii(src: &[u8]) -> Option<Cow<'_, str>> {
		use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};

		match Uts46::new().to_ascii(src, AsciiDenyList::STD3, Hyphens::CheckFirstLast, DnsLength::Verify) {
			Ok(Cow::Borrowed(x)) => x.strip_prefix(STUB).map(Cow::Borrowed),
			Ok(Cow::Owned(mut x)) =>
				if x.starts_with(STUB) {
					x.drain(..STUB.len());
					Some(Cow::Owned(x))
				}
				else { None },
			Err(_) => None,
		}
	}

	// Let's build the thing we'll be writing about building.
	let mut psl_main: RawMainMap = HashSet::with_capacity(10_000);
	let mut psl_wild: RawWildMap = HashMap::with_capacity(256);

	// Parse the raw data.
	let mut scratch = String::new();
	for mut line in psl_fetch_suffixes().lines() {
		line = line.trim_end();
		if line.is_empty() || line.starts_with("//") { continue; }

		// Figure out what kind of entry this is.
		let mut flags: u8 = 0;
		if let Some(rest) = line.strip_prefix('!') {
			line = rest;
			flags |= FLAG_EXCEPTION;
		}
		if let Some(rest) = line.strip_prefix("*.") {
			line = rest;
			flags |= FLAG_WILDCARD;
		}

		// To correctly handle the suffixes, we'll need to prepend a
		// hypothetical root and strip it off after cleanup.
		scratch.truncate(0);
		scratch.push_str(STUB);
		scratch.push_str(line);
		let Some(host) = idna_to_ascii(scratch.as_bytes()) else { continue; };

		// This is a wildcard exception.
		if 0 != flags & FLAG_EXCEPTION {
			if let Some((before, after)) = host.split_once('.') {
				let before = before.to_owned();
				if let Some(v) = psl_wild.get_mut(after) { v.push(before); }
				else {
					psl_wild.insert(after.to_owned(), vec![before]);
				}
			}
		}
		// This is the main wildcard entry.
		else if 0 != flags & FLAG_WILDCARD {
			psl_wild.entry(host.into_owned()).or_default();
		}
		// This is a normal suffix.
		else {
			psl_main.insert(host.into_owned());
		}
	}

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
	// Note: this needs to match the version exported by lib.rs!
	const AHASHER: ahash::RandomState = ahash::RandomState::with_seeds(
		0x8596_cc44_bef0_1aa0,
		0x98d4_0948_da60_19ae,
		0x49f1_3013_c503_a6aa,
		0xc4d7_82ff_3c9f_7bef,
	);
	AHASHER.hash_one(src)
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

#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
#[expect(unsafe_code, reason = "Content is ASCII.")]
/// # Stringify Map Keys.
///
/// This returns a comma-separated list of stringified (numeric) keys,
/// including `_` thousand separators in all the right places to appease
/// clippy.
fn nice_map_keys(map: Vec<u64>) -> String {
	/// # Decimals, 00-99.
	const DOUBLE: [[u8; 2]; 100] = [
		[48, 48], [48, 49], [48, 50], [48, 51], [48, 52], [48, 53], [48, 54], [48, 55], [48, 56], [48, 57],
		[49, 48], [49, 49], [49, 50], [49, 51], [49, 52], [49, 53], [49, 54], [49, 55], [49, 56], [49, 57],
		[50, 48], [50, 49], [50, 50], [50, 51], [50, 52], [50, 53], [50, 54], [50, 55], [50, 56], [50, 57],
		[51, 48], [51, 49], [51, 50], [51, 51], [51, 52], [51, 53], [51, 54], [51, 55], [51, 56], [51, 57],
		[52, 48], [52, 49], [52, 50], [52, 51], [52, 52], [52, 53], [52, 54], [52, 55], [52, 56], [52, 57],
		[53, 48], [53, 49], [53, 50], [53, 51], [53, 52], [53, 53], [53, 54], [53, 55], [53, 56], [53, 57],
		[54, 48], [54, 49], [54, 50], [54, 51], [54, 52], [54, 53], [54, 54], [54, 55], [54, 56], [54, 57],
		[55, 48], [55, 49], [55, 50], [55, 51], [55, 52], [55, 53], [55, 54], [55, 55], [55, 56], [55, 57],
		[56, 48], [56, 49], [56, 50], [56, 51], [56, 52], [56, 53], [56, 54], [56, 55], [56, 56], [56, 57],
		[57, 48], [57, 49], [57, 50], [57, 51], [57, 52], [57, 53], [57, 54], [57, 55], [57, 56], [57, 57]
	];

	#[inline]
	fn triple(idx: usize) -> [u8; 3] {
		let a = idx.wrapping_div(100) as u8 + b'0';
		let [b, c] = DOUBLE[idx % 100];
		[a, b, c]
	}

	// A buffer to hold stringified numbers. The initial value doesn't really
	// matter, but by starting with u64::MAX we can make sure that all commas
	// are in the right place and we have enough space for any u64.
	let mut buf: [u8; 26] = *b"18_446_744_073_709_551_615";
	let mut out = String::with_capacity(map.len() * 28); // Assume the worst; it'll be close.

	for mut num in map {
		let mut from = buf.len();

		// Stringify the trailing triplets first, if any.
		for chunk in buf.rchunks_exact_mut(4) {
			if 999 < num {
				chunk[1..].copy_from_slice(triple((num % 1000) as usize).as_slice());
				num /= 1000;
				from -= 4;
			}
			else { break; }
		}

		// Three more.
		if 99 < num {
			from -= 3;
			buf[from..from + 3].copy_from_slice(triple(num as usize).as_slice());
		}
		// Two more.
		else if 9 < num {
			from -= 2;
			buf[from..from + 2].copy_from_slice(DOUBLE[num as usize].as_slice());
		}
		// One more.
		else {
			from -= 1;
			buf[from] = (num as u8) + b'0';
		}

		// Safety: everything we've written/inherited will be an ASCII digit.
		out.push_str(unsafe {
			std::str::from_utf8_unchecked(&buf[from..])
		});
		out.push_str(", ");
	}

	// Remove the trailing punctuation.
	if out.ends_with(", ") { out.truncate(out.len() - 2); }

	out
}
