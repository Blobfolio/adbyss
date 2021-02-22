/*!
# Adbyss: Public Suffix - Build
*/

use ahash::{
	AHashMap,
	AHashSet,
};
use std::io::Write;



/// # Build Suffix RS.
///
/// This parses the raw lines of `public_suffix_list.dat` to build out valid
/// Rust code that can be included in `lib.rs`.
///
/// It's a bit ugly, but saves having to do this at runtime!
pub fn main() {
	println!("cargo:rerun-if-changed=skel/public_suffix_list.dat");

	// Let's build the thing we'll be writing about building.
	let mut psl_main: AHashSet<String> = AHashSet::new();
	let mut psl_wild: AHashMap<String, Vec<String>> = AHashMap::new();

	const FLAG_EXCEPTION: u8 = 0b0001;
	const FLAG_WILDCARD: u8  = 0b0010;

	// Parse the raw data.
	std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
		.map(|mut x| { x.push("skel/public_suffix_list.dat"); x })
		.and_then(std::fs::read_to_string)
		.expect("Unable to read public_suffix_list.dat")
		.lines()
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

	// Our generated script will live here.
	let mut file = std::env::var("OUT_DIR")
		.ok()
		.map(|mut x| { x.push_str("/adbyss-list.rs"); x })
		.and_then(|x| std::fs::File::create(x).ok())
		.expect("Unable to create public_suffix_list.rs");

	let (main_len, main_inserts) = build_psl_main(psl_main);
	let (wild_len, wild_inserts) = build_psl_wild(psl_wild);
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

/// # Build PSL_MAIN.
fn build_psl_main(set: AHashSet<String>) -> (usize, String) {
	let mut set: Vec<String> = set.iter()
		.map(|x| format!("\t\tout.insert(\"{}\");\n", x))
		.collect();
	set.sort();

	(
		set.len(),
		set.concat(),
	)
}

/// # Build PSL_WILD.
fn build_psl_wild(set: AHashMap<String, Vec<String>>) -> (usize, String) {
	let mut set: Vec<String> = set.iter()
		.map(|(k, v)| format!(
			"\t\tout.insert(\"{}\", vec![{}]);\n",
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
