/*!
# Adbyss: Public Suffix - Build
*/

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



type RawMainMap = HashSet<String>;
type RawWildMap = HashMap<String, Vec<String>>;



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
	for host in psl_wild.keys() {
		assert!(! psl_main.contains(host), "Duplicate host.");
	}

	// Reformat it.
	let (
		suffixes,
		suffix_from_slice,
		suffixes_wild,
		suffix_wild_arms,
		suffix_from_slice_len
	) = build_list(&psl_main, &psl_wild);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-list.rs"))
		.expect("Unable to create adbyss-list.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/list.rs.txt"),
		suffixes = suffixes,
		suffix_from_slice = suffix_from_slice,
		suffixes_wild = suffixes_wild,
		suffix_wild_arms = suffix_wild_arms,
		suffix_from_slice_len = suffix_from_slice_len,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
}

#[cfg(not(feature = "docs-workaround"))]
/// # Load Data.
fn load_data() -> (RawMainMap, RawWildMap) {
	// Let's build the thing we'll be writing about building.
	let mut psl_main: RawMainMap = HashSet::new();
	let mut psl_wild: RawWildMap = HashMap::new();

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
	let mut psl_main: RawMainMap = HashSet::new();
	psl_main.insert(String::from("com"));

	let mut psl_wild: RawWildMap = HashMap::new();
	psl_wild.insert(String::from("bd"), Vec::new());

	(psl_main, psl_wild)
}

/// # Build List.
///
/// This takes the lightly-processed main and wild lists, and generates all the
/// actual structures we'll be using within Rust.
fn build_list(main: &RawMainMap, wild: &RawWildMap) -> (String, String, String, String, String) {
	// enum name, byte equivalent, length, exception kind
	let mut opts: Vec<(String, String, usize, String)> = Vec::with_capacity(main.len() + wild.len());
	let mut lengths: Vec<usize> = Vec::new();

	// The main entries are easy.
	for host in main {
		let len = host.len();
		lengths.push(len);
		opts.push((
			format!("Psl{:04}", opts.len()),
			format!("b\"{}\"", host),
			len,
			String::new(),
		));
	}

	// The wildcards are a bit more dramatic.
	for (host, ex) in wild {
		let len = host.len();
		lengths.push(len);

		let ex =
			if ex.is_empty() { String::from("false") }
			else {
				let ex: String = format_wild_arm(ex);
				if ex.contains('|') { format!("matches!(src, {})", ex) }
				else { format!("src == {}", ex) }
			};

		opts.push((
			format!("Psl{:04}", opts.len()),
			format!("b\"{}\"", host),
			len,
			ex,
		));
	}

	opts.sort_by(|a, b| a.1.cmp(&b.1));
	lengths.sort_unstable();
	lengths.dedup();

	// Set up the inner from_slice functions, and build the arms of the outer
	// one.
	let mut arms: Vec<String> = Vec::with_capacity(lengths.len());
	let mut from_slice_len: Vec<String> = Vec::with_capacity(lengths.len());

	for len in lengths {
		let name = format!("from_slice{}", len);
		arms.push(format!("\t\t\t{} => Self::{}(src),", len, name));

		// Find out how many we're dealing with.
		let size: usize = opts.iter()
			.filter(|(_, _, l, _)| *l == len)
			.count();

		// If there's more than 100 entries in this section, let's split it
		// into two.
		if size > 200 {
			let start_bytes: Vec<u8> = opts.iter()
				.filter_map(|(_, b, l, _)|
					if *l == len { Some(b.as_bytes()[2]) }
					else { None }
				)
				.collect();
			assert_eq!(size, start_bytes.len());

			// Find a good byte to break on, starting at the midway point, and
			// going down as far as 1:2.
			let mut idx = (size / 2) + 1;
			let mut byte: u8 = 0;
			while idx >= size / 3 {
				if start_bytes[idx] != start_bytes[idx - 1] {
					byte = start_bytes[idx];
					break;
				}
				idx -= 1;
			}

			// Proceed with the complicated version if we found a good byte.
			if byte != 0 {
				let mut left: Vec<String> = Vec::new();
				let mut right: Vec<String> = Vec::new();
				for (e, b, l, _) in &opts {
					if *l == len {
						let line = format!("\t\t\t{} => Some(Self::{}),", b, e);
						if b.as_bytes()[2] < byte { left.push(line); }
						else { right.push(line); }
					}
				}
				left.push(String::from("\t\t\t_ => None,"));
				right.push(String::from("\t\t\t_ => None,"));

				from_slice_len.push(format!(
					"\tconst fn {}(src: &[u8]) -> Option<Self> {{\n\t\tif src[0] < {} {{ Self::{}a(src) }}\n\t\telse {{ Self::{}b(src) }}\n\t}}",
					name,
					byte,
					name,
					name,
				));
				from_slice_len.push(format!(
					"\tconst fn {}a(src: &[u8]) -> Option<Self> {{\n\t\tmatch src {{\n{}\n\t\t}}\n\t}}",
					name,
					left.join("\n"),
				));
				from_slice_len.push(format!(
					"\tconst fn {}b(src: &[u8]) -> Option<Self> {{\n\t\tmatch src {{\n{}\n\t\t}}\n\t}}",
					name,
					right.join("\n"),
				));

				continue;
			}
		}

		// Otherwise let's leave it be.
		let mut tmp: Vec<String> = Vec::new();
		for (e, b, l, _) in &opts {
			if *l == len {
				tmp.push(format!("\t\t\t{} => Some(Self::{}),", b, e));
			}
		}
		tmp.push(String::from("\t\t\t_ => None,"));

		from_slice_len.push(format!(
			"\tconst fn {}(src: &[u8]) -> Option<Self> {{\n\t\tmatch src {{\n{}\n\t\t}}\n\t}}",
			name,
			tmp.join("\n"),
		));
	}

	// Build the outer from_slice function.
	arms.push(String::from("\t\t\t_ => None,"));
	let from_slice: String = format!(
		"\tpub(super) const fn from_slice(src: &[u8]) -> Option<Self> {{\n\t\tmatch src.len() {{\n{}\n\t\t}}\n\t}}",
		arms.join("\n"),
	);
	let from_slice_len = from_slice_len.join("\n");

	// Build the definition list.
	let mut list: Vec<String> = opts.iter().map(|(e, _, _, _)| format!("\t{},", e)).collect();
	list.sort_unstable();
	let list: String = list.join("\n");

	// Build the wild-matching list!
	let mut wilds: HashMap<&str, Vec<String>> = HashMap::new();
	for (e, _, _, a) in &opts {
		if ! a.is_empty() && a != "false" {
			wilds.entry(a).or_insert_with(Vec::new).push(format!("Self::{}", e));
		}
	}
	let mut wilds: Vec<(Vec<String>, &str)> = wilds.into_iter()
		.map(|(k, v)| (v, k))
		.collect();
	wilds.sort_by(|a, b| a.0.len().cmp(&b.0.len()));
	let mut wild_arms: Vec<String> = wilds.into_iter()
		.map(|(hosts, arm)| format!("\t\t\t{} => {},", hosts.join(" | "), arm))
		.collect();
	wild_arms.push(String::from("\t\t\t_ => false,"));
	let wild_arms: String = wild_arms.join("\n");

	// Find the wild suffixes real quick.
	let mut wilds: Vec<String> = opts.iter()
		.filter_map(|(e, _, _, a)|
			if a.is_empty() { None }
			else { Some(format!("Self::{}", e)) }
		)
		.collect();
	wilds.sort_unstable();
	let wilds: String = wilds.join(" | ");

	(list, from_slice, wilds, wild_arms, from_slice_len)
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

/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	let mut out = std::fs::canonicalize(dir).expect("Missing OUT_DIR.");
	out.push(name);
	out
}
