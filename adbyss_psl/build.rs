/*!
# Adbyss: Public Suffix - Build
*/

use regex::Regex;
use std::{
	cmp::Ordering,
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
type RawIdna = Vec<(u32, u32, String, String)>;



const SUFFIX_URL: &str = "https://publicsuffix.org/list/public_suffix_list.dat";
const IDNA_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaMappingTable.txt";
const IDNA_TEST_URL: &str = "https://www.unicode.org/Public/idna/14.0.0/IdnaTestV2.txt";


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

	let (map_str, map_one_len, map_one, from_char) = idna_build(raw);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-idna.rs"))
		.expect("Unable to create adbyss-idna.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/idna.rs.txt"),
		map_str = map_str,
		map_one_len = map_one_len,
		map_one = map_one,
		from_char = from_char,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
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

fn idna_build(raw: RawIdna) -> (String, usize, String, String) {
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
	let mut map_one: Vec<String> = Vec::new();

	for (first, last, label, sub) in raw {
		// Single-character.
		if first == last {
			let ch = format!("'\\u{{{:x}}}'", first);
			if sub.is_empty() {
				map_one.push(format!("({}, CharKind::{})", ch, label));
			}
			else if let Some((lo, hi, len)) = find_map_str(&sub) {
				map_one.push(format!(
					"({}, CharKind::Mapped(MapIdx {{ a: {}, b: {}, l: {} }}))",
					ch, lo, hi, len
				));
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

	// For the single-char entries, we need to build up an array that we can
	// "extend" into the Rust HashMap.
	let map_one_len: usize = map_one.len();
	let map_one = map_one.chunks(1024)
		.map(|chunk| format!("\tout.extend([{}]);", chunk.join(", ")))
		.collect::<Vec<String>>()
		.join("\n");

	(map_str, map_one_len, map_one, from_char)
}

#[cfg(not(feature = "docs-workaround"))]
/// # Fetch IDNA.
///
/// This downloads the raw IDNA mapping table.
fn idna_fetch_data() -> String {
	// Cache this locally for up to an hour.
	let cache = out_path("IdnaMappingTable.txt");
	if let Some(x) = std::fs::metadata(&cache)
		.ok()
		.filter(Metadata::is_file)
		.and_then(|meta| meta.modified().ok())
		.and_then(|time| time.elapsed().ok().filter(|secs| secs.as_secs() < 3600))
		.and_then(|_| std::fs::read_to_string(&cache).ok())
	{
		return x;
	}

	println!("cargo:warning=IDNA data has to be downloaded.");

	// Download it fresh.
	let raw = ureq::get(IDNA_URL)
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(|e| e.into()))
		.expect("Unable to fetch IDNA data.");

	// We don't need to panic if the cache-save fails; the system is only
	// hurting its future self in such cases. ;)
	let _res = File::create(cache)
		.and_then(|mut file| file.write_all(raw.as_bytes()).and_then(|_| file.flush()));

	// Return the data.
	raw
}

#[cfg(feature = "docs-workaround")]
/// # Fetch IDNA.
///
/// This is a fake version that returns a static string with just enough data
/// to parse the structures. This is only used by Docs.rs, which doesn't
/// support network builds.
fn idna_fetch_data() -> String {
	String::from("# Fake.
0D60..0D61    ; valid                                  # 1.1  MALAYALAM LETTER VOCALIC RR..MALAYALAM LETTER VOCALIC LL
0D64..0D65    ; disallowed                             # NA   <reserved-0D64>..<reserved-0D65>
180B..180D    ; ignored                                # 3.0  MONGOLIAN FREE VARIATION SELECTOR ONE..MONGOLIAN FREE VARIATION SELECTOR THREE
17DD          ; valid                                  # 4.0  KHMER SIGN ATTHACAN
1C80          ; mapped                 ; 0432          # 9.0  CYRILLIC SMALL LETTER ROUNDED VE
2F9FE..2F9FF  ; mapped                 ; 980B          # 3.1  CJK COMPATIBILITY IDEOGRAPH-2F9FE..CJK COMPATIBILITY IDEOGRAPH-2F9FF")
}

#[cfg(not(feature = "docs-workaround"))]
/// # Fetch IDNA Tests.
///
/// This fetches the IDNA unit tests.
fn idna_fetch_test_data() -> String {
	// Cache this locally for up to an hour.
	let cache = out_path("IdnaTestV2.txt");
	if let Some(x) = std::fs::metadata(&cache)
		.ok()
		.filter(Metadata::is_file)
		.and_then(|meta| meta.modified().ok())
		.and_then(|time| time.elapsed().ok().filter(|secs| secs.as_secs() < 3600))
		.and_then(|_| std::fs::read_to_string(&cache).ok())
	{
		return x;
	}

	println!("cargo:warning=IDNA test data has to be downloaded.");

	// Download it fresh.
	let raw = ureq::get(IDNA_TEST_URL)
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(|e| e.into()))
		.expect("Unable to fetch IDNA test data.");

	// We don't need to panic if the cache-save fails; the system is only
	// hurting its future self in such cases. ;)
	let _res = File::create(cache)
		.and_then(|mut file| file.write_all(raw.as_bytes()).and_then(|_| file.flush()));

	// Return the data.
	raw
}

#[cfg(feature = "docs-workaround")]
/// # Fetch IDNA Tests.
///
/// This is a fake version that provides just enough data to build.
fn idna_fetch_test_data() -> String {
	String::from("fass.de; ; ; ; ; ;  # fass.de
faß.de; ; ; xn--fa-hia.de; ; fass.de;  # faß.de
Faß.de; faß.de; ; xn--fa-hia.de; ; fass.de;  # faß.de
xn--fa-hia.de; faß.de; ; xn--fa-hia.de; ; ;  # faß.de")
}

/// # Load Data.
fn idna_load_data() -> RawIdna {
	// First pass: parse each line, and group by type.
	let mut tbd: HashMap<String, RawIdna> = HashMap::new();
	for mut line in idna_fetch_data().lines().filter(|x| ! x.starts_with('#') && ! x.trim().is_empty()) {
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

/// # Load Data.
fn idna_load_test_data() -> Vec<(String, Option<String>)> {
	idna_fetch_test_data()
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

fn idna_parse_label(src: &str) -> Option<String> {
	match src {
		"valid" | "deviation" => Some(String::from("Valid")),
		"ignored"=> Some(String::from("Ignored")),
		"mapped" => Some(String::from("Mapped")),
		_ => None,
	}
}

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
		suffixes,
		suffix_from_slice,
		suffixes_wild,
		suffix_wild_arms,
		suffix_from_slice_len
	) = psl_build_list(&psl_main, &psl_wild);

	// Our generated script will live here.
	let mut file = File::create(out_path("adbyss-psl.rs"))
		.expect("Unable to create adbyss-psl.rs");

	// Save it!
	write!(
		&mut file,
		include_str!("./skel/psl.rs.txt"),
		suffixes = suffixes,
		suffix_from_slice = suffix_from_slice,
		suffixes_wild = suffixes_wild,
		suffix_wild_arms = suffix_wild_arms,
		suffix_from_slice_len = suffix_from_slice_len,
	)
		.and_then(|_| file.flush())
		.expect("Unable to save reference list.");
}

/// # Build List.
///
/// This takes the lightly-processed main and wild lists, and generates all the
/// actual structures we'll be using within Rust.
fn psl_build_list(main: &RawMainMap, wild: &RawWildMap) -> (String, String, String, String, String) {
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
				let ex: String = psl_format_wild_arm(ex);
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

				// Prioritize com, net, org.
				if len == 3 {
					let special = ["com", "net", "org"];
					left.sort_by(|a, b| {
						let a_special = special.contains(&&a[5..8]);
						let b_special = special.contains(&&b[5..8]);
						if a_special == b_special {
							a.cmp(b)
						}
						else if a_special { Ordering::Less }
						else { Ordering::Greater }
					});
					right.sort_by(|a, b| {
						let a_special = special.contains(&&a[5..8]);
						let b_special = special.contains(&&b[5..8]);
						if a_special == b_special {
							a.cmp(b)
						}
						else if a_special { Ordering::Less }
						else { Ordering::Greater }
					});
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
fn psl_fetch_suffixes() -> String {
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

	println!("cargo:warning=Suffix data has to be downloaded.");

	// Download it fresh.
	let raw = ureq::get(SUFFIX_URL)
		.set("user-agent", "Mozilla/5.0")
		.call()
		.and_then(|r| r.into_string().map_err(|e| e.into()))
		.map(|raw| {
			let re = Regex::new(r"(?m)^\s*").unwrap();
			re.replace_all(&raw, "").to_string()
		})
		.expect("Unable to fetch suffix data.");

	// We don't need to panic if the cache-save fails; the system is only
	// hurting its future self in such cases. ;)
	let _res = File::create(cache)
		.and_then(|mut file| file.write_all(raw.as_bytes()).and_then(|_| file.flush()));

	// Return the data.
	raw
}

/// # Format Exception Match Conditions.
fn psl_format_wild_arm(src: &[String]) -> String {
	let mut out: Vec<String> = src.iter()
		.map(|x| format!(r#"b"{}""#, x))
		.collect();
	out.sort();
	out.join(" | ")
}

#[cfg(not(feature = "docs-workaround"))]
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
/// # (Fake) Load Data.
///
/// This is a network-free workaround to allow `docs.rs` to be able to generate
/// documentation for this library.
///
/// Don't try to compile this library with the `docs-workaround` feature or the
/// library won't work properly.
fn psl_load_data() -> (RawMainMap, RawWildMap) {
	let mut psl_main: RawMainMap = HashSet::new();
	psl_main.insert(String::from("com"));

	let mut psl_wild: RawWildMap = HashMap::new();
	psl_wild.insert(String::from("bd"), Vec::new());

	(psl_main, psl_wild)
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
