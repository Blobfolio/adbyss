/*!
# Benchmark: `adbyss_psl::new`
*/

use brunch::{
	Bench,
	benches,
};
use adbyss_psl::Domain;
use std::time::Duration;

benches!(
	Bench::new("adbyss_psl::Domain", "new(blobfolio.com)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::new("blobfolio.com")),

	Bench::new("adbyss_psl::Domain", "new(www.blobfolio.com)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::new("www.blobfolio.com")),

	Bench::new("adbyss_psl::Domain", "new(食狮.com.cn)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::new("食狮.com.cn")),

	Bench::new("adbyss_psl::Domain", "new(xn--85x722f.com.cn)")
		.timed(Duration::from_secs(2))
		.with(|| Domain::new("xn--85x722f.com.cn")),
);
