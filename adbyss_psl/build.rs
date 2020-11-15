/*!
# `Adbyss PSL`: Build
*/

use ahash::{
	AHashMap,
	AHashSet,
};
use std::io::{
	BufWriter,
	Write,
};



/// # Build Suffix RS.
pub fn main() {
	println!("cargo:rerun-if-changed=skel/public_suffix_list.dat");

	// The folder holding our data.
	let skel_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
		.map(|mut x| { x.push("skel"); x })
		.ok()
		.filter(|x| x.is_dir())
		.expect("Missing skel directory.");

	// The raw public suffix data file.
	let mut raw_file = skel_dir.clone();
	raw_file.push("public_suffix_list.dat");
	assert!(raw_file.is_file(), "Missing public_suffix_list.dat.");

	// Let's build the thing we'll be writing about building.
	let mut psl_set: AHashSet<String> = AHashSet::new();
	let mut psl_wild: AHashMap<String, Vec<String>> = AHashMap::new();

	const FLAG_EXCEPTION: u8 = 0b0001;
	const FLAG_WILDCARD: u8  = 0b0010;

	// Parse the raw data.
	std::fs::read_to_string(&raw_file)
		.expect("Unable to read public_suffix_list.dat")
		.lines()
		//.take_while(|&line| line != "// ===END ICANN DOMAINS===")
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
				psl_set.insert(host);
			}
		);

	// Our generated script will live here.
	let mut script_file = skel_dir;
	script_file.push("public_suffix_list.rs");
	let mut file = BufWriter::new(
		std::fs::File::create(&script_file)
			.expect("Unable to create public_suffix_list.rs")
	);

	file.write_all(b"lazy_static::lazy_static! {\n").unwrap();

	// Handle the main set.
	{
		let mut tmp: Vec<String> = psl_set.drain().collect();
		tmp.sort();

		file.write_all(b"\t/// # Main Suffixes.\n").unwrap();
		file.write_all(b"\tstatic ref PSL_MAIN: AHashSet<&'static str> = {\n").unwrap();
		writeln!(
			&mut file,
			"\t\tlet mut out: AHashSet<&'static str> = AHashSet::with_capacity({});",
			tmp.len()
		).unwrap();

		tmp.iter().for_each(|line| {
			file.write_all(b"\t\tout.insert(\"").unwrap();
			file.write_all(line.as_bytes()).unwrap();
			file.write_all(b"\");\n").unwrap();
		});

		file.write_all(b"\t\tout\n\t};\n").unwrap();
	}

	// Handle the weird set.
	{
		let mut tmp: Vec<&str> = psl_wild.keys().map(String::as_str).collect();
		tmp.sort_unstable();

		file.write_all(b"\n\t/// # Weird Suffixes.\n").unwrap();
		file.write_all(b"\tstatic ref PSL_WILD: AHashMap<&'static str, Vec<&'static str>> = {\n").unwrap();
		writeln!(
			&mut file,
			"\t\tlet mut out: AHashMap<&'static str, Vec<&'static str>> = AHashMap::with_capacity({});",
			tmp.len()
		).unwrap();

		tmp.iter().for_each(|&k| {
			let mut v = psl_wild[k].clone();
			v.sort();
			writeln!(
				&mut file,
				r#"		out.insert("{}", vec![{}]);"#,
				k,
				v.iter()
					.map(|x| format!(r#""{}""#, x))
					.collect::<Vec<String>>()
					.join(", ")
			).unwrap();
		});

		file.write_all(b"\t\tout\n\t};\n").unwrap();
	}

	// Finish it off!
	file.write_all(b"}\n").unwrap();
	file.flush().unwrap();
}
