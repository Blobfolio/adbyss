/*!
# Adbyss: Writing!
*/

use adbyss_psl::Domain;
use crate::{
	AdbyssError,
	MAX_LINE,
};
use dactyl::NiceU64;
use std::{
	collections::BTreeMap,
	fmt,
	net::IpAddr,
	path::Path,
};
use trimothy::TrimMut;
use utc2k::FmtUtc2k;



/// # Start Marker.
const MARKER_START: &str = "##########\n# ADBYSS #\n##########";

/// # End Marker.
const MARKER_END: &str = "## End of Adbyss Rules ##\n";



/// # Shitlist.
pub(super) enum Shitlist {
	/// # Flat List: One Domain Per Line.
	Flat(Vec<Domain>),

	/// # Compact List: Grouped by TLD.
	Compact(Vec<Domain>),
}

impl Shitlist {
	/// # Into Vec.
	///
	/// Return the inner domain list. Note this is always "flat".
	pub(super) fn into_vec(self) -> Vec<Domain> {
		match self { Self::Flat(s) | Self::Compact(s) => s }
	}

	/// # Length.
	pub(super) fn len(&self) -> usize {
		match self { Self::Flat(s) | Self::Compact(s) => s.len() }
	}

	/// # Estimate Write Length.
	///
	/// Calculate the approximate number of bytes required to write all the host
	/// entries so we can reserve accordingly.
	///
	/// Note: this should be slightly more than we'd actually need.
	fn estimate_byte_len(&self) -> usize {
		let raw: &[Domain] = match self {
			Self::Flat(s) | Self::Compact(s) => s.as_slice(),
		};
		raw.iter().fold(400_usize, |acc, d| acc + d.len() + 9)
	}
}

impl Shitlist {
	/// # Prune Custom Hosts.
	///
	/// Parse the custom host entries from the raw (adbyss-free) hosts file and
	/// remove them from the shitlist, if present.
	pub(super) fn prune_custom_hosts(&mut self, raw: &str) {
		// Borrow the set.
		let set: &mut Vec<Domain> = match self { Self::Flat(s) | Self::Compact(s) => s };

		// Split lines.
		for line in raw.trim().lines() {
			// Trim whitespace and comments.
			let mut line = line.trim();
			if let Some(pos) = line.find('#') { line = line[..pos].trim_end(); }

			// Split into words. If the first is an IP, try the rest to see if
			// they're prunable domains.
			let mut words = line.split_ascii_whitespace();
			if words.next().map_or(false, |w| w.parse::<IpAddr>().is_ok()) {
				for word in words.filter_map(Domain::new) {
					if let Ok(pos) = set.binary_search(&word) { set.remove(pos); }
				}
			}
		}
	}

	/// # Append to String.
	///
	/// Append the Adbyss section header, shitlist, and footer to the end of
	/// the hosts string.
	pub(super) fn append(&self, hosts: &mut String) -> Result<(), fmt::Error> {
		use fmt::Write;
		hosts.try_reserve(self.estimate_byte_len()).map_err(|_| fmt::Error)?;
		write!(hosts, "\n{}\n{self}\n{MARKER_END}", ShitlistHeader(self.len()))
	}
}

impl fmt::Display for Shitlist {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			// Flat is easy!
			Self::Flat(list) => {
				for v in list { writeln!(f, "0.0.0.0 {v}")?; }
			},
			// Compact requires some extra throught…
			Self::Compact(list) => {
				// First, let's reorganize the entries by TLD.
				let mut grouped = BTreeMap::<&str, Vec<&Domain>>::new();
				for v in list {
					grouped.entry(v.tld()).or_default().push(v);
				}

				// Now print the TLDs, though we might need to split if they
				// run too long.
				for group in grouped.into_values() {
					let mut line_len = 0;
					f.write_str("0.0.0.0")?;
					for v in group {
						line_len += v.len() + 1;

						// Continue the current line.
						if line_len <= MAX_LINE { write!(f, " {v}")?; }
						// Start a new line.
						else {
							line_len = v.len() + 1;
							write!(f, "\n0.0.0.0 {v}")?;
						}
					}
					f.write_str("\n")?;
				}
			},
		}

		Ok(())
	}
}



/// # Shitlist Header.
///
/// This is used to print a pretty marker/header to identify the list. The
/// footer, by contrast, is just a single line, so `MARKER_END` is sufficient.
struct ShitlistHeader(usize);

impl fmt::Display for ShitlistHeader {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// No notes for no output.
		if self.0 == 0 { writeln!(f, "{MARKER_START}") }
		else {
			let len = NiceU64::from(self.0);
			let now = FmtUtc2k::now();
			writeln!(f, "{MARKER_START}
#
# This section is automatically generated. Don't make any changes here or
# they'll just get blown away the next time Adbyss is run.
#
# If you have custom host entries, add them before or after this section.
#
# Updated: {now} UTC
# Blocked: {len} garbage hosts
#
# Eat the rich.
#
##########")
		}
	}
}



/// # Read Hostfile.
///
/// Read the local hostfile, stripping out any Adbyss-related entries.
pub(super) fn read_hosts<P: AsRef<Path>>(src: P) -> Result<(String, bool), AdbyssError> {
	// Read the file.
	let src = src.as_ref();
	let mut out = std::fs::read_to_string(src)
		.map_err(|_| AdbyssError::Read(src.to_string_lossy().into_owned()))?;

	// Strip out Adbyss parts.
	let mut any = false;
	let len_end = MARKER_END.len();
	let len_start = MARKER_START.len();

	// Look for start markers.
	while let Some(start) = out.find(MARKER_START) {
		any = true;

		// If we have an end marker, trim from the start of start to the end
		// of end.
		if let Some(end) = out[start + len_start..].find(MARKER_END) {
			out.replace_range(start..end + start + len_start + len_end, "");
		}
		// Otherwise we can simply truncate the file at the start of start and
		// call it a day.
		else {
			out.truncate(start);
			break;
		}
	}

	// Look for stray end markers.
	// TODO: use remove_matches once stable.
	while let Some(end) = out.find(MARKER_END) {
		any = true;
		out.replace_range(end..end + len_end, "");
	}

	// Clean up the end a bit.
	out.trim_end_mut();
	out.push('\n');

	// Return it!
	Ok((out, any))
}

/// # Write to File.
///
/// Hosts files can be weird; they may not like atomic writes. This falls back
/// to regular create/write instead, but will return an error if both fail.
pub(super) fn write_to_file(dst: &Path, data: &[u8]) -> Result<(), AdbyssError> {
	use std::io::Write;

	write_atomic::write_file(dst, data)
		.or_else(|_| std::fs::File::create(dst).and_then(|mut file|
			file.write_all(data).and_then(|()| file.flush())
		))
		.map_err(|_| AdbyssError::Write(dst.to_string_lossy().into_owned()))
}



#[cfg(test)]
mod test {
	use super::*;
	use std::collections::BTreeSet;

	#[test]
	fn t_shitlist_fmt() {
		let set: BTreeSet<Domain> = BTreeSet::from([
			Domain::new("blobfolio.com").unwrap(),
			Domain::new("facebook.com").unwrap(),
			Domain::new("google.com").unwrap(),
			Domain::new("www.blobfolio.com").unwrap(),
			Domain::new("www.google.com").unwrap(),
			Domain::new("www1.blobfolio.com").unwrap(),
			Domain::new("www10.blobfolio.com").unwrap(),
			Domain::new("www11.blobfolio.com").unwrap(),
			Domain::new("www12.blobfolio.com").unwrap(),
			Domain::new("www13.blobfolio.com").unwrap(),
			Domain::new("www2.blobfolio.com").unwrap(),
			Domain::new("www3.blobfolio.com").unwrap(),
			Domain::new("www4.blobfolio.com").unwrap(),
			Domain::new("www5.blobfolio.com").unwrap(),
			Domain::new("www6.blobfolio.com").unwrap(),
			Domain::new("www7.blobfolio.com").unwrap(),
			Domain::new("www8.blobfolio.com").unwrap(),
			Domain::new("www9.blobfolio.com").unwrap(),
		]);

		// Flat.
		assert_eq!(
			Shitlist::Flat(set.iter().cloned().collect()).to_string(),
			"0.0.0.0 blobfolio.com
0.0.0.0 facebook.com
0.0.0.0 google.com
0.0.0.0 www.blobfolio.com
0.0.0.0 www.google.com
0.0.0.0 www1.blobfolio.com
0.0.0.0 www10.blobfolio.com
0.0.0.0 www11.blobfolio.com
0.0.0.0 www12.blobfolio.com
0.0.0.0 www13.blobfolio.com
0.0.0.0 www2.blobfolio.com
0.0.0.0 www3.blobfolio.com
0.0.0.0 www4.blobfolio.com
0.0.0.0 www5.blobfolio.com
0.0.0.0 www6.blobfolio.com
0.0.0.0 www7.blobfolio.com
0.0.0.0 www8.blobfolio.com
0.0.0.0 www9.blobfolio.com
",
		);

		// Compact.
		assert_eq!(
			Shitlist::Compact(set.iter().cloned().collect()).to_string(),
			"0.0.0.0 blobfolio.com www.blobfolio.com www1.blobfolio.com www10.blobfolio.com www11.blobfolio.com www12.blobfolio.com www13.blobfolio.com www2.blobfolio.com www3.blobfolio.com www4.blobfolio.com www5.blobfolio.com www6.blobfolio.com www7.blobfolio.com
0.0.0.0 www8.blobfolio.com www9.blobfolio.com
0.0.0.0 facebook.com
0.0.0.0 google.com www.google.com
",
		);
	}

	#[test]
	fn t_read_hosts() {
		// Strip the adbyss chunks from the test hosts file.
		let (stub, changed) = read_hosts("skel/test-full.hosts")
			.expect("Failed to read hosts stub.");
		assert!(changed, "The host should have gotten stripped.");
		assert_eq!(stub, include_str!("../skel/test-stripped.hosts"));
	}

	#[test]
	fn t_prune_hosts() {
		let mut list = Shitlist::Flat(vec![
			Domain::new("analytics.com").unwrap(),
			Domain::new("blobfolio.com").unwrap(),
			Domain::new("yahoo.com").unwrap(),
		]);
		let (stub, _) = read_hosts("skel/test-full.hosts")
			.expect("Failed to read hosts stub.");

		// Parse and prune the stub hosts.
		list.prune_custom_hosts(&stub);

		// That should have removed blobfolio.com.
		assert_eq!(
			list.into_vec(),
			&[
				Domain::new("analytics.com").unwrap(),
				Domain::new("yahoo.com").unwrap(),
			]
		);
	}

	#[test]
	fn t_append() {
		// Flat.
		let list = Shitlist::Flat(vec![
			Domain::new("analytics.com").unwrap(),
			Domain::new("blobfolio.com").unwrap(),
			Domain::new("www.blobfolio.com").unwrap(),
			Domain::new("yahoo.com").unwrap(),
		]);

		let (mut stub, _) = read_hosts("skel/test-full.hosts")
			.expect("Failed to read hosts stub.");

		list.append(&mut stub).expect("Append failed!");
		assert!(stub.contains(MARKER_START));
		assert!(stub.contains("0.0.0.0 analytics.com"));
		assert!(stub.contains("0.0.0.0 blobfolio.com"));
		assert!(stub.contains("0.0.0.0 www.blobfolio.com"));
		assert!(stub.contains("0.0.0.0 yahoo.com"));
		assert!(stub.contains(MARKER_END));

		// Compact.
		let list = Shitlist::Compact(vec![
			Domain::new("analytics.com").unwrap(),
			Domain::new("blobfolio.com").unwrap(),
			Domain::new("www.blobfolio.com").unwrap(),
			Domain::new("yahoo.com").unwrap(),
		]);

		let (mut stub, _) = read_hosts("skel/test-full.hosts")
			.expect("Failed to read hosts stub.");

		list.append(&mut stub).expect("Append failed!");
		assert!(stub.contains(MARKER_START));
		assert!(stub.contains("0.0.0.0 analytics.com"));
		assert!(stub.contains("0.0.0.0 blobfolio.com www.blobfolio.com"));
		assert!(stub.contains("0.0.0.0 yahoo.com"));
		assert!(stub.contains(MARKER_END));
	}
}
