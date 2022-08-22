/*!
# Benchmark: `adbyss_psl::new`
*/

use brunch::{
	Bench,
	benches,
};
use adbyss_psl::Domain;

benches!(
	Bench::new("adbyss_psl::Domain::new(blobfolio.com)")
		.run(|| Domain::new("blobfolio.com")),

	Bench::new("adbyss_psl::Domain::new(www.blobfolio.com)")
		.run(|| Domain::new("www.blobfolio.com")),

	Bench::new("adbyss_psl::Domain::new(食狮.com.cn)")
		.run(|| Domain::new("食狮.com.cn")),

	Bench::new("adbyss_psl::Domain::new(xn--85x722f.com.cn)")
		.run(|| Domain::new("xn--85x722f.com.cn")),
);
