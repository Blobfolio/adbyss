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

	let (map_str, map, map_len) = idna_build(raw);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-idna.rs"))
		.expect("Unable to create adbyss-idna.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/idna.rs.txt"),
		map_str = map_str,
		map = map,
		map_len = map_len,
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
fn idna_build(mut raw: RawIdna) -> (String, String, usize) {
	// Build a map substitution string containing each possible substitution,
	// with the occasional overlap allowed.
	let map_str: Vec<char> = idna_crunch_superstring(raw.iter()
		.filter_map(|(_, _, _, sub)|
			if sub.is_empty() { None }
			else { Some(sub.as_str()) }
		)
	);

	// This just lets us quickly find the indexes and lengths of strings in the
	// `map_str` table we created above.
	let find_map_str = |src: &str| -> (u8, u8, u8) {
		let needle: Vec<char> = src.chars().collect();
		for (idx, w) in map_str.windows(needle.len()).enumerate() {
			if w == needle {
				let idx = idx as u16;
				let [lo, hi] = idx.to_le_bytes();
				return (lo, hi, needle.len() as u8);
			}
		}

		// This shouldn't happen; we've already asserted all mappings are
		// present.
		panic!("Mising mapping {}", src);
	};

	// Update the mappings.
	for (first, last, label, sub) in &mut raw {
		assert!(first <= last, "Invalid range.");
		if ! sub.is_empty() {
			let (lo, hi, len) = find_map_str(sub);
			*label = IdnaLabel::Mapped(lo, hi, len);
		}
	}

	// Reformat!
	let mut map: Vec<(u32, Option<u32>, IdnaLabel)> = raw.into_iter()
		.filter_map(|(first, last, label, _)|
			// We'll specialize these common cases.
			if
				(first == '-' as u32 && last == '.' as u32) ||
				(first == 'a' as u32 && last == 'z' as u32) ||
				(first == '0' as u32 && last == '9' as u32)
			{ None }
			else if first == last { Some((first, None, label)) }
			else { Some((first, Some(last), label)) }
		)
		.collect();

	let map_len: usize = map.len();
	map.sort_by(|a, b| a.0.cmp(&b.0));

	// Reformat again, this time for output.
	// Format the array.
	let map = format!(
		"static MAP: [(u32, Option<NonZeroU32>, CharKind); {}] = [{}];",
		map_len,
		map.into_iter()
			.map(|(first, last, label)|
				if let Some(last) = last {
					format!(
						"({}, Some(unsafe {{ NonZeroU32::new_unchecked({}) }}), {})",
						format_u32(first),
						format_u32(last),
						label,
					)
				}
				else { format!("({}, None, {})", format_u32(first), label) }
			)
			.collect::<Vec<String>>()
			.join(", "),
	);

	// Reformat MAP_STR one last time into an array with proper char notation
	// to keep the linter happy.
	let map_str: String = format!(
		"static MAP_STR: [char; {}] = [{}];",
		map_str.len(),
		map_str.into_iter()
			.map(|c|
				if c.is_ascii() { format!("'{}'", c) }
				else { format!("'\\u{{{:x}}}'", c as u32) }
			)
			.collect::<Vec<String>>()
			.join(", ")
	);

	// Done!
	(map_str, map, map_len)
}

/*
/// # Build Ranged Code.
///
/// This compiles the ranged matches into a sorted array, which we jump
/// through at runtime using a custom binary search function.
fn idna_build_ranged(mut raw: Vec<(u32, u32, IdnaLabel)>) -> (usize, String) {
	// Sort!
	raw.sort_by(|a, b| a.0.cmp(&b.0));

	// Build up an array object.
	let len = raw.len();
	let entries: Vec<String> = raw.into_iter()
		.map(|(start, end, kind)| format!(
			"({}_u32, {}_u32, {})",
			format_u32(start),
			format_u32(end),
			kind
		))
		.collect();

	(
		len,
		format!(
			"static RANGES: [(u32, u32, CharKind); {}] = [{}];",
			entries.len(),
			entries.join(", "),
		),
	)
}*/

/// # Parse Superstring.
///
/// Because we're representing the IDNA/Unicode remappings as indexes of a
/// single static string, we can save a lot of space by overlapping repeated
/// ranges.
///
/// This method takes the raw replacement strings and:
/// * Deduplicates them;
/// * Strips out entries that exist as substrings of other entries;
/// * Calculates an optimal pair-joining by looking at the overlap potential of each pair's end/start characters;
///
/// This has to strike a balance between computation time and total savings,
/// so it does not compare all possible orderings (as that would take days),
/// but even so, it still manages to reduce the overall size by about 40%.
fn idna_crunch_superstring<'a, I>(set: I) -> Vec<char>
where I: IntoIterator<Item = &'a str> {
	let mut set: Vec<String> = set.into_iter()
		.map(String::from)
		.collect::<HashSet<String>>()
		.into_iter()
		.collect();
	let old = set.clone();

	// Strip entries that can be represented as substrings of other entries.
	set.retain(|x| old.iter().all(|y| y == x || ! y.contains(x)));

	// Sort the list by the longest records first to give us a consistent
	// starting point from run-to-run.
	set.sort_by(|a, b| match b.len().cmp(&a.len()) {
		std::cmp::Ordering::Equal => a.cmp(b),
		cmp => cmp,
	});

	// We're going to be working with chars a lot, so let's pre-compute them
	// for each entry. At the same time, we can also separate out the single-
	// char entries, which by process of elimination, have no overlap.
	let (singles, mut set): (Vec<Vec<char>>, Vec<Vec<char>>) = set.into_iter()
		.map(|x| x.chars().collect::<Vec<char>>())
		.partition(|a| a.len() == 1);

	// Loop the loop the loop!
	while set.len() > 1 {
		// Examine all pairs to see how much overlap exists between the end of
		// the first with the beginning of the second. If any, we'll store the
		// amount saved along with the relevant indexes for later.
		let mut saved: Vec<(usize, usize, usize)> = Vec::with_capacity(set.len());
		for i in 0..set.len() {
			for j in 0..set.len() {
				if i == j { continue; }

				// Figure out the lengths.
				let a_len = set[i].len();
				let b_len = set[j].len();
				let len = usize::min(a_len, b_len);

				// How much overlap is there?
				for diff in (1..len).rev() {
					if set[i].iter().skip(a_len - diff).eq(set[j].iter().take(diff)) {
						saved.push((diff, i, j));
						break;
					}
				}
			}
		}

		// We're done!
		if saved.is_empty() { break; }

		// Sort saved by total savings desc, total size desc, alpha asc. Again,
		// this helps give us consistent results from run-to-run.
		saved.sort_by(|a, b| match b.0.cmp(&a.0) {
			std::cmp::Ordering::Equal => {
				let a_len = set[a.1].len() + set[a.2].len() - a.0;
				let b_len = set[b.1].len() + set[b.2].len() - b.0;
				match b_len.cmp(&a_len) {
					std::cmp::Ordering::Equal => set[a.1].cmp(&set[b.1]),
					cmp => cmp,
				}
			},
			cmp => cmp,
		});

		// Find the round's highest savings.
		let best: usize = saved[0].0;

		// Build a new set by joining all of the biggest-saving combinations,
		// then adding the rest as-were. We need to keep track of the indexes
		// we've hit along the way so we don't accidentally add anything twice.
		let mut new: Vec<Vec<char>> = Vec::with_capacity(set.len());
		let mut seen: HashSet<usize> = HashSet::with_capacity(set.len());

		for (diff, left, right) in saved {
			// Join and push any pairing with savings matching the round's
			// best. Because the same indexes might appear twice independently
			// of one another, we have to do `seen.contains()` matching rather
			// than straight inserts, or else we might lose one.
			if diff == best && ! seen.contains(&left) && ! seen.contains(&right) {
				seen.insert(left);
				seen.insert(right);

				// Join right onto left, then steal left for the new vector.
				let mut joined = set[left].clone();
				joined.extend(set[right].iter().skip(diff).copied());
				new.push(joined);
			}
			// Because we've sorted by savings, we can stop looking once the
			// savings change.
			else if diff != best { break; }
		}

		// Now we need to loop through the original set, adding any entries
		// (as-are) that did not get joined earlier. We'll also skip any
		// entries which now happen to appear as substrings within the new set.
		for (idx, line) in set.iter().enumerate() {
			if seen.insert(idx) {
				new.push(line.clone());
			}
		}

		// Swap set and new so we can do this all over again!
		std::mem::swap(&mut set, &mut new);
	}

	// Flatten into a string.
	let mut flat: String = String::with_capacity(11_000);
	for line in singles {
		flat.extend(line.into_iter());
	}
	for line in set {
		let line: String = line.into_iter().collect();
		if ! flat.contains(&line) {
			flat.push_str(&line);
		}
	}

	// Make sure we didn't lose anything along the way.
	for entry in old {
		assert!(flat.contains(&entry), "Missing mapping: {}", entry);
	}

	flat.chars().collect()
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

		let label = match IdnaLabel::from_str(line[1]) {
			Some(x) => x,
			None => continue,
		};

		let sub =
			if matches!(label, IdnaLabel::Valid | IdnaLabel::Ignored) { String::new() }
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
	let mut out: RawIdna = Vec::with_capacity(8192);
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

	// Hold map key/value pairs.
	let mut map: Vec<(u64, String)> = Vec::with_capacity(main.len() + wild.len());

	// Populate this with stringified tuples (bytes=>kind).
	for host in main {
		// We'll prioritize these.
		if host == "com" || host == "net" || host == "org" { continue; }
		let hash = hash_tld(host.as_bytes());
		map.push((hash, String::from("SuffixKind::Tld")));
	}
	for (host, ex) in wild {
		let hash = hash_tld(host.as_bytes());
		if ex.is_empty() {
			map.push((hash, String::from("SuffixKind::Wild")));
		}
		else {
			let ex = psl_format_wild(ex);
			let ex = wild_map.get(&ex).expect("Missing wild arm.");
			map.push((hash, format!("SuffixKind::WildEx(WildKind::{})", ex)));
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
		"/// # Map Keys.\nstatic MAP_K: [u64; {}] = [{}];\n\n/// # Map Values.\nstatic MAP_V: [SuffixKind; {}] = [{}];",
		len,
		map_keys.into_iter()
			.map(format_u64)
			.collect::<Vec<String>>()
			.join(", "),
		len,
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
	Mapped(u8, u8, u8),
}

impl std::fmt::Display for IdnaLabel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Valid => f.write_str("CharKind::Valid"),
			Self::Ignored => f.write_str("CharKind::Ignored"),
			Self::Mapped(a, b, l) => write!(
				f,
				"CharKind::Mapped({}, {}, {})",
				a,
				b,
				l,
			),
		}
	}
}

impl IdnaLabel {
	fn from_str(src: &str) -> Option<Self> {
		match src {
			"valid" | "deviation" => Some(IdnaLabel::Valid),
			"ignored"=> Some(IdnaLabel::Ignored),
			"mapped" => Some(IdnaLabel::Mapped(0, 0, 0)),
			_ => None,
		}
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
	let mut hasher = ahash::AHasher::new_with_keys(1319, 2371);
	hasher.write(src);
	hasher.finish()
}

/// # Format U32.
///
/// This formats a `u32` with `_` separators for the thousands.
fn format_u32(src: u32) -> String {
	NiceU32::from(src).as_str().replace(",", "_")
}

/// # Format U64.
///
/// This formats a `u64` with `_` separators for the thousands.
fn format_u64(src: u64) -> String {
	NiceU64::from(src).as_str().replace(",", "_")
}
