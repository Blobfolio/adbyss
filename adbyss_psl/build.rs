/*!
# Adbyss: Public Suffix - Build
*/

use dactyl::{
	NiceU32,
	NiceU64,
};
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
	hash::Hasher,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};



type RawIdna = Vec<(u32, u32, String, String)>;
type RawMainMap = HashSet<String>;
type RawWildMap = HashMap<String, Vec<String>>;



const IDNA_TEST_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaTestV2.txt";
const IDNA_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaMappingTable.txt";
const SUFFIX_URL: &str = "https://publicsuffix.org/list/public_suffix_list.dat";



/// # Build Resources!
///
/// This monstrous build script downloads and parses the raw suffix and IDNA
/// datasets and writes them into Rust scripts our library can directly
/// include.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=skel/idna.rs.txt");
	println!("cargo:rerun-if-changed=skel/psl.rs.txt");

	idna();
	idna_tests();
	psl();
}

/// # Build IDNA Table.
///
/// Pull down the IDNA/PUNYCODE data to build out valid Rust code.
fn idna() {
	let raw = idna_load_data();
	assert!(! raw.is_empty(), "Missing IDNA data.");

	let (map_str, map, from_char) = idna_build(raw);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-idna.rs"))
		.expect("Unable to create adbyss-idna.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/idna.rs.txt"),
		map_str = map_str,
		map = map,
		from_char = from_char,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
}

fn idna_build(raw: RawIdna) -> (String, String, String) {
	// Build a map substitution string.
	let mut map_str = String::new();
	for (_, _, _, sub) in &raw {
		if ! sub.is_empty() && ! map_str.contains(sub) {
			map_str.push_str(sub);
		}
	}

	let find_map_str = |src: &str| -> Option<(u8, u8, u8)> {
		let idx = map_str.find(src)? as u16;
		let [lo, hi] = idx.to_le_bytes();
		let len = src.len() as u8;
		Some((lo, hi, len))
	};

	// For now, these will hold patterns x..=y.
	let mut valid: Vec<String> = Vec::new();
	let mut ignored: Vec<String> = Vec::new();

	// These are grouped by branch, just in case more than one branch is shared
	// between discontiguous points. type=>[x..=y]
	let mut mapped: HashMap<String, Vec<String>> = HashMap::new();

	// Single-char entries are stored as (char, type).
	let mut builder = CharMap::default();

	for (first, last, label, sub) in raw {
		// Single-character.
		if first == last {
			if sub.is_empty() {
				builder.insert(first, format!("CharKind::{}", label));
			}
			else if let Some((lo, hi, len)) = find_map_str(&sub) {
				builder.insert(first, format!("CharKind::Mapped(MapIdx {{ a: {}, b: {}, l: {} }})", lo, hi, len));
			}

			continue;
		}

		// Range.
		match label.as_str() {
			"Valid" => valid.push(format!(
				"'\\u{{{:x}}}'..='\\u{{{:x}}}'",
				first,
				last,
			)),
			"Ignored" => ignored.push(format!(
				"'\\u{{{:x}}}'..='\\u{{{:x}}}'",
				first,
				last,
			)),
			_ => {
				if let Some((lo, hi, len)) = find_map_str(&sub) {
					let key = format!(
						"Self::Mapped(MapIdx {{ a: {}, b: {}, l: {} }})",
						lo,
						hi,
						len,
					);

					let rg = format!(
						"'\\u{{{:x}}}'..='\\u{{{:x}}}'",
						first,
						last,
					);

					mapped.entry(key).or_insert_with(Vec::new).push(rg);
				}
			},
		}
	}

	// We actually need to re-format the map string so it uses char notation or
	// else the linter would complain.
	let map_str: String = map_str.chars()
		.map(|c|
			if c.is_ascii() { String::from(c) }
			else { format!("\\u{{{:x}}}", c as u32) }
		)
		.collect();

	// Turn the ranged values into arms.
	let valid = format!("\t\t\t{} => Some(Self::Valid),", valid.join(" | "));
	let ignored = format!("\t\t\t{} => Some(Self::Ignored),", ignored.join(" | "));
	let mapped = mapped.into_iter()
		.map(|(k, e)| format!("\t\t\t{} => Some({}),", e.join(" | "), k))
		.collect::<Vec<String>>()
		.join("\n");

	let from_char = format!("{}\n{}\n{}", ignored, valid, mapped);

	(map_str, builder.build(), from_char)
}

/// # Load Data.
fn idna_load_data() -> RawIdna {
	// First pass: parse each line, and group by type.
	let mut tbd: HashMap<String, RawIdna> = HashMap::new();
	for mut line in download("IdnaMappingTable.txt", IDNA_URL).lines().filter(|x| ! x.starts_with('#') && ! x.trim().is_empty()) {
		// Strip comments.
		if let Some(idx) = line.bytes().position(|b| b == b'#') {
			line = &line[..idx];
		}

		let line: Vec<&str> = line.split(';').map(|x| x.trim()).collect();
		if line.len() < 2 || line[0].is_empty() || line[1].is_empty() {
			continue;
		}

		let (first, last) = match idna_parse_range(line[0]) {
			Some(x) => x,
			None => continue,
		};

		let label = match idna_parse_label(line[1]) {
			Some(x) => x,
			None => continue,
		};

		let sub =
			if label != "Mapped" { String::new() }
			else if let Some(sub) = line.get(2) {
				sub.split_ascii_whitespace()
					.map(|x| u32::from_str_radix(x, 16).expect("Invalid u32."))
					.map(|x| char::from_u32(x).expect("Invalid char."))
					.collect()
			}
			else { continue };

		// Group everything by type.
		tbd.entry(label.clone()).or_insert_with(Vec::new).push((
			first, last, label, sub
		));
	}

	// Second pass: merge ranges by type, and compile into one mass set.
	let mut out: RawIdna = Vec::new();
	for (_, mut set) in tbd {
		set.sort_by(|a, b| a.0.cmp(&b.0));

		for idx in 1..set.len() {
			// If this is a continuation, adjust the range and move on.
			if set[idx - 1].1 + 1 == set[idx].0 && set[idx - 1].3 == set[idx].3 {
				set[idx].0 = set[idx - 1].0;

				// If this is the end, we have to push it.
				if idx + 1 == set.len() {
					out.push(set[idx].clone());
				}
				continue;
			}

			// Push the previous range.
			out.push(set[idx - 1].clone());

			// If this is the end, let's push it too.
			if idx + 1 == set.len() {
				out.push(set[idx].clone());
			}
		}
	}

	// Third pass: sort out one more time, just in case.
	out.sort_by(|a, b| a.0.cmp(&b.0));
	out
}

/// # Parse Labels.
///
/// This condenses the various IDNA labels into the succinct set used by our
/// library.
fn idna_parse_label(src: &str) -> Option<String> {
	match src {
		"valid" | "deviation" => Some(String::from("Valid")),
		"ignored"=> Some(String::from("Ignored")),
		"mapped" => Some(String::from("Mapped")),
		_ => None,
	}
}

/// # Parse Range.
///
/// This parses a hex range (or single value) into proper pairs of u32.
fn idna_parse_range(src: &str) -> Option<(u32, u32)> {
	let (first, last) =
		if src.contains("..") {
			let mut split = src.split("..");
			let first = u32::from_str_radix(split.next()?, 16).ok()?;
			let last = u32::from_str_radix(split.next()?, 16).ok()?;
			(first, last)
		}
		else {
			let first = u32::from_str_radix(src, 16).ok()?;
			(first, first)
		};

	if char::from_u32(first).is_some() && char::from_u32(last).is_some() {
		Some((first, last))
	}
	else { None }
}



/// # Build IDNA Tests.
///
/// The spec is big and terrible; we want to make sure we're testing as many
/// edge cases as possible to avoid bugs.
fn idna_tests() {
	let raw = idna_load_test_data();
	assert!(! raw.is_empty(), "Missing IDNA data.");

	let out: String = raw.into_iter()
		.map(|(i, mut o)| {
			format!("{} {}", i, o.take().unwrap_or_default())
		})
		.collect::<Vec<String>>()
		.join("\n");

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-idna-tests.rs"))
		.expect("Unable to create adbyss-idna.rs");
	file.write_all(out.as_bytes())
		.and_then(|_| file.flush())
		.expect("Failed to save IDNA tests.");
}

/// # Load Data.
fn idna_load_test_data() -> Vec<(String, Option<String>)> {
	download("IdnaTestV2.txt", IDNA_TEST_URL)
		.lines()
		.filter(|x| ! x.starts_with('#') && ! x.trim().is_empty())
		.filter_map(|mut line| {
			// Strip comments.
			if let Some(idx) = line.bytes().position(|b| b == b'#') {
				line = &line[..idx];
			}

			let config = idna::Config::default()
				.use_std3_ascii_rules(true)
				.verify_dns_length(true)
				.check_hyphens(true);

			let line: Vec<&str> = line.split(';').map(|x| x.trim()).collect();
			if ! line.is_empty() && ! line[0].is_empty() {
				let input = line[0].to_string();
				let output = config.to_ascii(input.trim_matches(|c| c == '.'))
					.ok()
					.filter(|x|
						! x.is_empty() &&
						! x.starts_with('.') &&
						! x.ends_with('.') &&
						x.contains('.')
					);
				Some((input, output))
			}
			else { None }
		})
		.collect()
}



/// # Build Suffix RS.
///
/// This parses the raw lines of `public_suffix_list.dat` to build out valid
/// Rust code that can be included in `lib.rs`.
///
/// It's a bit ugly, but saves having to do this at runtime!
fn psl() {
	// Pull the raw data.
	let (psl_main, psl_wild) = psl_load_data();
	assert!(! psl_main.is_empty(), "No generic PSL entries found.");
	assert!(! psl_wild.is_empty(), "No wildcard PSL entries found.");
	for host in psl_wild.keys() {
		assert!(! psl_main.contains(host), "Duplicate host.");
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
/// This takes the lightly-processed main and wild lists, and generates all the
/// actual structures we'll be using within Rust.
fn psl_build_list(main: &RawMainMap, wild: &RawWildMap) -> (String, String, String) {
	// The wild stuff is the hardest.
	let (wild_map, wild_kinds, wild_arms) = psl_build_wild(wild);

	// Populate this with stringified tuples (bytes=>kind).
	let mut builder = SuffixMap::default();
	for host in main {
		// We'll prioritize these.
		if host == "com" || host == "net" || host == "org" { continue; }
		let hash = hash_tld(host.as_bytes());
		builder.insert(hash, String::from("SuffixKind::Tld"));
	}
	for (host, ex) in wild {
		let hash = hash_tld(host.as_bytes());
		if ex.is_empty() {
			builder.insert(hash, String::from("SuffixKind::Wild"));
		}
		else {
			let ex = psl_format_wild(ex);
			let ex = wild_map.get(&ex).expect("Missing wild arm.");
			builder.insert(hash, format!("SuffixKind::WildEx(WildKind::{})", ex));
		}
	}

	(builder.build(), wild_kinds, wild_arms)
}

/// # Build Wild Enum.
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
		let name = format!("Ex{}", k);
		wild_kinds.push(format!("{},", name));
		wild_map.insert(v, name);
	}

	// If there aren't any wild exceptions, we can just return an empty
	// placeholder that will never be referenced.
	if wild_kinds.is_empty() {
		return (wild_map, String::from("\tNone,"), String::from("\t\t\tSelf::None => false,"));
	}

	let wild_kinds: String = wild_kinds.join("\n");
	let mut wild_arms: Vec<(&String, &String)> = wild_map.iter().collect();
	wild_arms.sort_by(|a, b| a.1.cmp(b.1));
	let wild_arms = wild_arms.into_iter()
		.map(|(cond, name)| format!("\t\t\tSelf::{} => {},", name, cond))
		.collect::<Vec<String>>()
		.join("\n");

	(wild_map, wild_kinds, wild_arms)
}

/// # Fetch Suffixes.
///
/// This downloads and lightly cleans the raw public suffix list.
fn psl_fetch_suffixes() -> String {
	let raw = download("public_suffix_list.dat", SUFFIX_URL);
	let re = Regex::new(r"(?m)^\s*").unwrap();
	re.replace_all(&raw, "").to_string()
}

/// # Format Wild Exceptions.
///
/// This converts an array of exception hosts into a consistent string.
fn psl_format_wild(src: &[String]) -> String {
	if src.is_empty() { String::new() }
	else if src.len() == 1 {
		format!("src == b\"{}\"", src[0])
	}
	else {
		format!(
			"[{}].contains(src)",
			src.iter()
				.map(|s| format!("b\"{}\"", s))
				.collect::<Vec<String>>()
				.join(", ")
		)
	}
}

/// # Load Data.
fn psl_load_data() -> (RawMainMap, RawWildMap) {
	// Let's build the thing we'll be writing about building.
	let mut psl_main: RawMainMap = HashSet::new();
	let mut psl_wild: RawWildMap = HashMap::new();

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
/// # (FAKE) Download File.
///
/// This is a workaround for Docs.rs that pulls a local (saved) copy of the
/// file rather than fetching the latest from a remote source.
fn download(name: &str, _url: &str) -> String {
	let mut path = PathBuf::from("./skel/raw");
	path.push(name);
	match std::fs::read_to_string(path) {
		Ok(x) => x,
		Err(_) => panic!("Unable to load {}.", name),
	}
}

#[cfg(not(feature = "docs-workaround"))]
/// # Download File.
///
/// This downloads and caches a remote data file used by the build.
fn download(name: &str, url: &str) -> String {
	// Cache this locally for up to an hour.
	let cache = out_path(name);
	if let Some(x) = try_cache(&cache) {
		return x;
	}

	// Download it fresh.
	let raw: String = match ureq::get(url)
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(|e| e.into())) {
		Ok(x) => x,
		Err(_) => panic!("Unable to download {}.", name),
	};

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

/// # Try Cache.
fn try_cache(path: &Path) -> Option<String> {
	std::fs::metadata(path)
		.ok()
		.filter(Metadata::is_file)
		.and_then(|meta| meta.modified().ok())
		.and_then(|time| time.elapsed().ok().filter(|secs| secs.as_secs() < 3600))
		.and_then(|_| std::fs::read_to_string(path).ok())
}



/// # Helper: Static Map.
///
/// This is a crude static hash map builder. Content will be split into static
/// arrays (buckets) called MAP0, MAP1, etc., and a companion map_get() method
/// will be generated to fetch matches.
macro_rules! map_builder {
	($name: ident, $ty:ty, $dactyl:ty, $k_type: ty, $v_type: ty, $hash_func:literal) => (
		#[derive(Default)]
		/// # Static Map.
		struct $name {
			set: Vec<($ty, String)>,
		}

		impl $name {
			/// # Insert.
			fn insert(&mut self, hash: $ty, val: String) {
				self.set.push((hash, val));
			}

			/// # Build.
			fn build(&mut self) -> String {
				let (buckets, bucket_coords, consolidated) = self.build_buckets();

				// Start building the code!
				let mut code: Vec<String> = Vec::new();

				// Push the maps!
				for (idx, coords) in bucket_coords.iter().enumerate() {
					let slice = &consolidated[coords.0..coords.1];
					code.push(format!(
						"/// # Map Data!\nstatic MAP{}: [({}, {}); {}] = [{}];",
						idx,
						stringify!($ty),
						stringify!($v_type),
						slice.len(),
						slice.join(", "),
					));
				}

				// And lastly, the searcher!
				{
					let mut tmp: Vec<String> = Vec::new();
					for idx in 0..bucket_coords.len() {
						// Last entry.
						if idx == bucket_coords.len() - 1 {
							tmp.push(format!(
								"\t\t_ => MAP{}.binary_search_by_key(&hash, |(a, _)| *a).ok().map(|idx| MAP{}[idx].1),",
								idx,
								idx,
							));
						}
						// All other entries.
						else {
							tmp.push(format!(
								"\t\t{} => MAP{}.binary_search_by_key(&hash, |(a, _)| *a).ok().map(|idx| MAP{}[idx].1),",
								idx,
								idx,
								idx,
							));
						}
					}

					code.push(format!(
						"/// # Search Map!\nfn map_get(src: {}) -> Option<{}> {{
	{}
	match hash % {} {{
{}
	}}
}}",
						stringify!($k_type),
						stringify!($v_type),
						$hash_func,
						buckets,
						tmp.join("\n"),
					));
				}

				// We're done!
				code.join("\n\n")
			}

			/// # Build Buckets and Consolidated Data.
			fn build_buckets(&mut self) -> ($ty, Vec<(usize, usize)>, Vec<String>) {
				if self.set.is_empty() { panic!("The static map set cannot be empty."); }
				{
					let tmp: HashSet<_> = self.set.iter().map(|(k, _)| *k).collect();
					if tmp.len() != self.set.len() {
						panic!("The static map contains duplicate keys!");
					}
				}

				// Come up with some buckets. We'll try to wind up with about
				// 128 entries per bucket, but we don't want more than 64 total
				// buckets.
				let buckets = <$ty>::try_from(self.set.len().wrapping_div(128))
					.expect("Bucket size exceeds hash size.")
					.min(64)
					.max(1);

				// Sort the data into said buckets.
				let mut bucket_coords: Vec<(usize, usize)> = Vec::new();
				let mut consolidated: Vec<String> = Vec::new();
				let mut out: Vec<Vec<($ty, &str)>> = Vec::new();
				out.resize_with(buckets as usize, Vec::new);
				for (k, v) in &self.set {
					let k: $ty = *k;
					let bucket = (k % buckets) as usize;
					out[bucket].push((k, v));
				}
				for inner in &mut out {
					inner.sort_by(|a, b| a.0.cmp(&b.0));
				}

				for tmp in out {
					let from = consolidated.len();
					for (k, v) in tmp {
						consolidated.push(format!(
							"({}_{}, {})",
							<$dactyl>::from(k).as_str().replace(",", "_"),
							stringify!($ty),
							v
						));
					}
					let to = consolidated.len();
					bucket_coords.push((from, to));
				}

				assert_eq!(bucket_coords.len(), buckets as usize, "Bucket count mismatch.");
				assert_eq!(consolidated.len(), self.set.len(), "Conslidated bucket mismatch.");

				(buckets, bucket_coords, consolidated)
			}
		}
	);
}

map_builder!(SuffixMap, u64, NiceU64, &[u8], SuffixKind, "
	use std::hash::Hasher;
	let mut hasher = ahash::AHasher::new_with_keys(1319, 2371);
	hasher.write(src);
	let hash = hasher.finish();
");
map_builder!(CharMap, u32, NiceU32, char, CharKind, "let hash = src as u32;");

/// # Hash TLD.
///
/// This is just a simple wrapper to convert a slice into a u64, used by the
/// suffix map builder.
fn hash_tld(src: &[u8]) -> u64 {
	let mut hasher = ahash::AHasher::new_with_keys(1319, 2371);
	hasher.write(src);
	hasher.finish()
}
