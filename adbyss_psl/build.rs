/*!
# Adbyss: Public Suffix - Build
*/

use std::{
	borrow::Cow,
	collections::{
		BTreeMap,
		BTreeSet,
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
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
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
	let out = psl_build(&psl_main, &psl_wild);

	// Our generated script will live here.
	File::create(out_path("adbyss-psl.rs"))
		.and_then(|mut file|
			file.write_all(out.as_bytes()).and_then(|()| file.flush())
		)
		.expect("Unable to save reference list.");
}

/// # Codegen.
///
/// This method crunches the (pre-filtered) Public Suffix data into static maps
/// we can query at runtime, and provides methods for querying them.
///
/// Ultimately, there are three kinds of entries:
/// * TLD: a normal TLD.
/// * Wild: a TLD that comprises both the explicit entry, as well as any arbitrary "subdomain".
/// * Wild-But: a Wild entry that contains one or more exceptions to chunks that may precede it.
fn psl_build(main: &RawMainMap, wild: &RawWildMap) -> String {
	use std::fmt::Write;

	// Pre-stringify and count the wildcard exceptions. There should only be
	// a couple of them.
	let wild_ex: BTreeMap<String, usize> = wild.values()
		.filter_map(|ex| psl_format_wild(ex))
		.collect::<BTreeSet<String>>()
		.into_iter()
		.enumerate()
		.map(|(v, k)| (k, v))
		.collect();

	// Combine the main and wild data into a single, deduped map, sorted for
	// binary search compatibility.
	assert!(main.contains("com"), "Normal tld list missing COM!");
	let mut map: BTreeMap<&str, Cow<str>> = main.iter()
		.map(|host| (host.as_str(), Cow::Borrowed("SuffixKind::Tld")))
		.chain(
			wild.iter().map(|(host, ex)|
				if ex.is_empty() { (host.as_str(), Cow::Borrowed("SuffixKind::Wild")) }
				else {
					// Allocating a new string for the lookup sucks, but there
					// aren't many exceptions so this doesn't run often.
					let ex = psl_format_wild(ex).and_then(|ex| wild_ex.get(&ex))
						.expect("Missing wild arm.");
					(host.as_str(), Cow::Owned(format!("SuffixKind::WildEx(WildKind::Ex{ex})")))
				}
			)
		)
		.collect();

	// Almost half of all domain registrations use .com; we specialize its
	// matching so can remove it from the lookup map.
	map.remove("com");

	// Make sure we didn't lose anything.
	assert_eq!(
		map.len(),
		main.len() + wild.len() - 1,
		"Main and Wild maps have overlapping keys!",
	);

	// Reorganize the list by part count since we'll have to split domains into
	// parts while searching anyway.
	let mut grouped: BTreeMap<usize, BTreeMap<&str, &str>> = BTreeMap::new();
	for (k, v) in map.iter() {
		let count = k.split('.').count();
		grouped.entry(count).or_default().insert(k, v);
	}

	// Generate the MAX_PARTS const so we know how much suffix splitting to
	// do when searching for matches. Note that this is one larger than the
	// discovered count because wildcards require an extra part.
	let mut out = String::with_capacity(365_000);
	writeln!(
		&mut out,
		"/// # Max Parts (Matchable).
const MAX_PARTS: usize = {};
",
		grouped.keys().last().unwrap() + 1
	).unwrap();

	// Generate MAP_## const for each of the grouped suffix/kind sets.
	for (count, set) in grouped.iter() {
		writeln!(
			&mut out,
			"/// # TLDs w/ {count} Part(s).
const MAP_{count}: &[(&[u8], SuffixKind)] = &[",
		).unwrap();
		let set = set.iter().collect::<Vec<_>>();
		for chunk in set.chunks(256) {
			out.push('\t');
			for (k, v) in chunk {
				write!(&mut out, "(b{k:?}, {v}), ").unwrap();
			}
			out.truncate(out.len() - 1);
			out.push('\n');
		}
		out.push_str("];\n\n");
	}

	// Generate the WildKind enum.
	out.push_str(r#"#[derive(Clone, Copy)]
/// # Wild Kind Exceptions.
pub(super) enum WildKind {"#);
	for ex in wild_ex.values() {
		writeln!(&mut out, "\n\t/// # Exception #{ex}.\n\tEx{ex},").unwrap();
	}
	out.push_str("}

impl WildKind {
	/// # Is Exception?
	pub(super) const fn is_match(self, src: &[u8]) -> bool {
		match self {
");
	for (cond, ex) in wild_ex.iter() {
		writeln!(&mut out, "\t\t\tSelf::Ex{ex} => {cond},").unwrap();
	}
	out.push_str("\t\t}
	}
}");

	// Lastly, generate a method to perform suffix-to-kind searches (using the
	// previously-generated maps).
	out.push_str(
"impl SuffixKind {
	/// # Suffix Kind From Slice.
	///
	/// Match a suffix from a byte slice, e.g. `com`.
	pub(super) fn from_parts(src: &[u8], parts: usize) -> Option<Self> {
		let map = match parts {\n");
	for count in grouped.keys() {
		// The aforementioned specialization for .com goes here!
		if *count == 1 {
			out.push_str("\t\t\t1 =>
\t\t\t\tif src == b\"com\" { return Some(Self::Tld); }
\t\t\t\telse { MAP_1 },\n");
		}
		// Otherwise just map the map.
		else {
			writeln!(&mut out, "\t\t\t{count} => MAP_{count},").unwrap();
		}
	}
	out.push_str("\t\t\t_ => return None,
\t\t};
\t\tlet pos = map.binary_search_by_key(&src, |(a, _)| a).ok()?;
\t\tSome(map[pos].1)
\t}\n}\n\n");

	// Return everything!
	out
}

/// # Fetch Suffixes.
///
/// This loads and lightly cleans the raw Public Suffix List data.
fn psl_fetch_suffixes() -> String {
	let mut raw = std::fs::read_to_string("skel/raw/public_suffix_list.dat")
		.expect("Unable to load public_suffix_list.dat");

	// Remove leading whitespace at the start of each line.
	let mut last = '\n';
	raw.retain(|c: char|
		if last == '\n' {
			if c.is_whitespace() { false }
			else {
				last = c;
				true
			}
		}
		else {
			last = c;
			true
		}
	);

	raw
}

/// # Format Wild Exceptions.
///
/// This builds the match condition for a wildcard's exceptions, if any.
/// (There aren't very many.)
fn psl_format_wild(src: &[String]) -> Option<String> {
	use std::fmt::Write;

	if src.is_empty() { None }
	else {
		let mut out = "matches!(src, ".to_owned();
		let mut iter = src.iter();
		let next = iter.next()?;
		write!(&mut out, "b\"{next}\"").ok()?;
		for next in iter {
			write!(&mut out, " | b\"{next}\"").ok()?;
		}
		out.push(')');
		Some(out)
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



/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	let mut out = std::fs::canonicalize(dir).expect("Missing OUT_DIR.");
	out.push(name);
	out
}
