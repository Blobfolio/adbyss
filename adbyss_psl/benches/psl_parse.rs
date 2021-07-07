/*!
# Benchmark: `adbyss_psl::parse`
*/

use brunch::{
	Bench,
	benches,
};
use adbyss_psl::Domain;
use std::time::Duration;

benches!(
	Bench::new("adbyss_psl::Domain", "parse(blobfolio.com)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::parse("blobfolio.com")),

	Bench::new("adbyss_psl::Domain", "parse(www.blobfolio.com)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::parse("www.blobfolio.com")),

	Bench::new("adbyss_psl::Domain", "parse(another.sub.domain.blobfolio.com)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::parse("another.sub.domain.blobfolio.com"))
);
