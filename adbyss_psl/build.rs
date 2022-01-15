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



type RawIdna = Vec<(u32, u32, IdnaLabel, String)>;
type RawMainMap = HashSet<String>;
type RawWildMap = HashMap<String, Vec<String>>;



const IDNA_TEST_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaTestV2.txt";
const IDNA_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaMappingTable.txt";
const SUFFIX_URL: &str = "https://publicsuffix.org/list/public_suffix_list.dat";



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
	println!("cargo:rerun-if-changed=skel/idna.rs.txt");
	println!("cargo:rerun-if-changed=skel/psl.rs.txt");

	idna();
	idna_tests();
	psl();
}

/// # Build IDNA/Unicode Table.
///
/// This method handles all operations related to the IDNA/Unicode table.
/// Ultimately, it collects a bunch of Rust "code" represented as strings, and
/// writes them to a pre-formatted template. The generated script is then
/// included by the library.
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

/// # Crunch IDNA/Unicode Table.
///
/// This builds:
/// * A static hash map of all single-character entries (valid, ignored, mapped);
/// * A single static string containing all possible mapping replacements. There are a few duplicates, so keeping it contiguous cuts down on the size of the data.
/// * A nested `match` branch to identify ranged-character entries.
fn idna_build(raw: RawIdna) -> (String, String, String) {
	// Build a map substitution string containing each possible substitution,
	// with the occasional overlap allowed.
	let map_str: String = {
		// First build a list of all the unique replacements.
		let mut map_str: Vec<&str> = raw.iter()
			.filter_map(|(_, _, _, sub)|
				if sub.is_empty() { None }
				else { Some(sub.as_ref()) }
			)
			.collect::<HashSet<&str>>()
			.into_iter()
			.collect::<Vec<&str>>();

		// Since we can overlap repeated ranges, starting with the longest
		// string first is a good, simple compression strategy.
		map_str.sort_by(|a, b|
			match b.len().cmp(&a.len()) {
				std::cmp::Ordering::Equal => a.cmp(b),
				cmp => cmp,
			}
		);

		// Build up a contiguous slice, ignoring any substrings that are
		// already represented anywhere within.
		let mut out = String::new();
		for line in map_str {
			if ! out.contains(line) {
				out.push_str(line);
			}
		}
		out
	};

	// This just lets us quickly find the indexes and lengths of strings in the
	// `map_str` table we created above.
	let find_map_str = |src: &str| -> Option<(u8, u8, u8)> {
		let idx = map_str.find(src)? as u16;
		let [lo, hi] = idx.to_le_bytes();
		let len = src.len() as u8;
		Some((lo, hi, len))
	};

	// Single-char entries are stored as (char, type).
	let mut builder = CharMap::default();

	// Ranged values are handled separately.
	let mut ranged: Vec<(u32, u32, String)> = Vec::new();

	// Separate the raw data into single and ranged entries.
	for (first, last, label, sub) in raw {
		// Single.
		if first == last {
			if sub.is_empty() {
				builder.insert(first, label.global().to_string());
			}
			else if let Some((lo, hi, len)) = find_map_str(&sub) {
				builder.insert(
					first,
					format!("CharKind::Mapped(MapIdx {{ a: {}, b: {}, l: {} }})", lo, hi, len)
				);
			}

			continue;
		}

		// Skip the following very common ranges; we'll specialize them!
		if
			(first == '-' as u32 && last == '.' as u32) ||
			(first == 'a' as u32 && last == 'z' as u32) ||
			(first == '0' as u32 && last == '9' as u32)
		{ continue; }

		// Ranged.
		let rg_label =
			match label {
				IdnaLabel::Valid | IdnaLabel::Ignored => label.local().to_string(),
				IdnaLabel::Mapped =>
					if let Some((lo, hi, len)) = find_map_str(&sub) {
						format!(
							"Self::Mapped(MapIdx {{ a: {}, b: {}, l: {} }})",
							lo,
							hi,
							len,
						)
					}
					else { continue; },
			};
		ranged.push((first, last, rg_label));
	}

	// We actually need to re-format the map string so it uses char notation or
	// else the linter will complain.
	let map_str: String = map_str.chars()
		.map(|c|
			if c.is_ascii() { String::from(c) }
			else { format!("\\u{{{:x}}}", c as u32) }
		)
		.collect();

	// Done!
	(map_str, builder.build(), idna_build_ranged(ranged))
}

/// # Build Ranged Code.
///
/// This compiles a nested `match` to efficiently determine whether or not a
/// given character falls within any of the 1000 or so discrete ranges the
/// IDNA/Unicode table comes with.
///
/// The outer arms represent the absolute min/max of its inner arms, and the
/// inner arms are grouped by output kind (valid, ignored, mapped).
///
/// Nesting is a pain in the ass, but cuts down on processing time by around
/// 20%.
fn idna_build_ranged(mut raw: Vec<(u32, u32, String)>) -> String {
	// Sort!
	raw.sort_by(|a, b| a.0.cmp(&b.0));

	// Come up with some buckets. We'll try to wind up with about
	// 128 entries per bucket, but we don't want more than 32 total
	// buckets. The actual number of buckets may be +/- one, but will be at
	// least one.
	let buckets = u32::try_from(raw.len().wrapping_div(128))
		.expect("Bucket size exceeds hash size.")
		.min(32)
		.max(1);

	// Unlike the other static maps, this one is branched in an ordered fashion
	// rather than using any hashing trickery.
	let chunks = raw.len() / buckets as usize;
	if chunks == 0 { panic!("Not enough entries for buckets."); }
	let mut out: Vec<Vec<(u32, u32, String)>> = raw.chunks(chunks)
		.map(|x| x.to_vec())
		.collect();

	// Find the ranges for each bucket, lowest low and highest high.
	let ranges: Vec<(u32, u32)> = out.iter()
		.map(|x| {
			let min = x.iter().map(|(n, _, _)| *n).min().unwrap();
			let max = x.iter().map(|(_, n, _)| *n).max().unwrap();
			(min, max)
		})
		.collect();

	// Now build out the nested arms. The min/max `ranges` form the outer arms,
	// and each pattern that falls within that range makes up the inner arms.
	let mut arms: Vec<String> = Vec::new();
	for (low, high) in ranges {
		// We need to resort the inners by output kind.
		let mut inner: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
		for (first, last, kind) in out.remove(0) {
			inner.entry(kind).or_insert_with(Vec::new).push((first, last));
		}

		// Move it back to a vec so we can sort it properly.
		let mut inner: Vec<(u32, String)> = inner.into_iter()
			.map(|(kind, set)| {
				let first = set[0].0;
				let set = set.into_iter()
					.map(|(f, l)| format!(
						"{}_u32..={}_u32",
						NiceU32::from(f).as_str().replace(",", "_"),
						NiceU32::from(l).as_str().replace(",", "_"),
					))
					.collect::<Vec<String>>()
					.join(" | ");
				(first, format!("\t\t\t\t{} => Some({}),", set, kind))
			})
			.collect();
		inner.sort_by(|a, b| a.0.cmp(&b.0));
		inner.push((0, String::from("\t\t\t\t_ => None,")));

		// Finally push it to the arm!
		arms.push(format!(
			"\t\t\t{}_u32..={}_u32 => match ch {{\n{}\n\t\t\t}},",
			NiceU32::from(low).as_str().replace(",", "_"),
			NiceU32::from(high).as_str().replace(",", "_"),
			inner.into_iter()
				.map(|(_, x)| x)
				.collect::<Vec<String>>()
				.join("\n"),
		));
	}

	arms.join("\n")
}

/// # Load Data.
///
/// This loads the raw IDNA/Unicode table data. With the exception of `docs.rs`
/// builds — which just pull a stale copy included with this library — the data
/// is downloaded fresh from `unicode.org`.
///
/// At the moment, version `14.0.0` is used, but that will change as new
/// standards are released.
fn idna_load_data() -> RawIdna {
	// First pass: parse each line, and group by type.
	let mut tbd: HashMap<IdnaLabel, RawIdna> = HashMap::new();
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
			if label != IdnaLabel::Mapped { String::new() }
			else if let Some(sub) = line.get(2) {
				sub.split_ascii_whitespace()
					.map(|x| u32::from_str_radix(x, 16).expect("Invalid u32."))
					.map(|x| char::from_u32(x).expect("Invalid char."))
					.collect()
			}
			else { continue };

		// Group everything by type.
		tbd.entry(label).or_insert_with(Vec::new).push((
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
/// The raw IDNA/Unicode tables have a lot of different types to accommodate
/// different standards releases (backward compatibility, etc.). Because this
/// library is highly opinionated, we can boil them down to just three kinds:
/// * Valid: the character can be passed through as-is.
/// * Ignored: the character is silently discarded.
/// * Mapped: the character is transformed into one or more alternative characters.
fn idna_parse_label(src: &str) -> Option<IdnaLabel> {
	match src {
		"valid" | "deviation" => Some(IdnaLabel::Valid),
		"ignored"=> Some(IdnaLabel::Ignored),
		"mapped" => Some(IdnaLabel::Mapped),
		_ => None,
	}
}

/// # Parse Range.
///
/// The raw IDNA/Unicode tables represent characters as 32-bit hex, either
/// individually — a single code point — or as a range. This method tries to
/// tease the true `u32` values from such strings.
///
/// This method always returns a start and (inclusive) end. If this represents
/// a single value rather than a range, the start and end will be equal.
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
/// The IDNA/Unicode spec is very complicated and it is incredibly easy to mess
/// up the parsing, particularly when PUNY decoding or encoding is required.
///
/// Thankfully, they publish comprehensive unit tests with each version.
///
/// Unfortunately, the format isn't easily digestible, so this method attempts
/// to parse and normalize it. The `idna` crate is used (during build) to
/// provide a trusted second opinion on what a given string _should_ parse to.
///
/// As our library only deals with ASCII, `idna` is made to crunch in that
/// mode.
///
/// It is worth noting that there are a couple "extra" things we do, so a few
/// of the tests will be discarded.
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
///
/// For typical builds, this downloads the raw unit tests from `unicode.org`,
/// but for `docs.rs`, a stale copy provided by this library is used instead.
///
/// As mentioned in [`idna_tests`] above, the `idna` crate is leveraged to
/// give us a trusted second opinion on how the lines should be parsed,
/// however there are a couple cases where Adbyss intentionally disagrees;
/// those tests are simply discarded.
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
/// * Wild-But: a Wild entry that contains one or more exceptions to chunks that may preceed it.
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
/// This downloads and lightly cleans the raw Public Suffix List data, except
/// when building for `docs.rs`, in which case it just grabs (and lightly
/// cleans) a stale copy included with the library.
///
/// The "cleaning" is really just a simple line trim.
fn psl_fetch_suffixes() -> String {
	let raw = download("public_suffix_list.dat", SUFFIX_URL);
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
				.map(|s| format!("b\"{}\"", s))
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
/// This is a workaround for `docs.rs`, which does not support network activity
/// during the build process. This library ships with stale copies of each data
/// file required by the library. This version of the [`download`] method just
/// pulls them straight from disk.
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
/// This downloads and caches a remote data file used by the build. There are
/// three difference sources we need to pull:
/// * Public Suffix List
/// * IDNA/Unicode tables
/// * IDNA/Unicode unit tests
///
/// The files get cached locally in the `target` directory for up to an hour to
/// keep network traffic from being obnoxious during repeated builds. If a
/// cached entry outlives that hour, or if the `target` directory is cleaned,
/// it will just download it anew.
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
///
/// The downloaded files are cached locally in the `target` directory, but we
/// don't want to run the risk of those growing stale if they persist between
/// sessions, etc.
///
/// At the moment, cached files are used if they are less than an hour old,
/// otherwise the cache is ignored and they're downloaded fresh.
fn try_cache(path: &Path) -> Option<String> {
	std::fs::metadata(path)
		.ok()
		.filter(Metadata::is_file)
		.and_then(|meta| meta.modified().ok())
		.and_then(|time| time.elapsed().ok().filter(|secs| secs.as_secs() < 3600))
		.and_then(|_| std::fs::read_to_string(path).ok())
}



#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
/// # IDNA Label.
///
/// This enum is used to associate individual IDNA/Unicode entries by type
/// without having to rely on string slices. These entries correspond to the
/// [`CharKind`] type published in the library, except that the indexes for
/// [`IdnaLabel::Mapped`] have to be specified separately (during later
/// processing).
enum IdnaLabel {
	Valid,
	Ignored,
	Mapped,
}

impl IdnaLabel {
	/// # Self Label.
	fn local(self) -> &'static str {
		match self {
			Self::Valid => "Self::Valid",
			Self::Ignored => "Self::Ignored",
			Self::Mapped => "Self::Mapped",
		}
	}

	/// # Global Label.
	fn global(self) -> &'static str {
		match self {
			Self::Valid => "CharKind::Valid",
			Self::Ignored => "CharKind::Ignored",
			Self::Mapped => "CharKind::Mapped",
		}
	}
}



/// # Helper: Static Map.
///
/// Both the Public Suffix and IDNA/Unicode data require gigantic lookup tables
/// to do their thing at runtime. In order to avoid the runtime cost of
/// instantiating a dynamic lookup table — e.g. a `Lazy<HashMap>` — we build
/// these statically.
///
/// The approach is similar to that used by `phf`, except we avoid the structs,
/// and push each bucket to its own static array. A lookup method is generated
/// with all the branching logic hard-coded, so it's really quite zippy!
///
/// Anyhoo, this macro generates a struct used by the builder to keep track of
/// the entries and generate the appropriate `Rust` code when the time comes.
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
///
/// In testing, the `ahash` algorithm is far and away the fastest, so that is
/// what we use, both during build and at runtime (i.e. search needles) during
/// lookup matching.
fn hash_tld(src: &[u8]) -> u64 {
	let mut hasher = ahash::AHasher::new_with_keys(1319, 2371);
	hasher.write(src);
	hasher.finish()
}
