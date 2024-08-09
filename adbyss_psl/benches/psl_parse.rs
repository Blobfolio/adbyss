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

	Bench::spacer(),

	Bench::new("adbyss_psl::Domain::try_from::<str>(blobfolio.com)")
		.run(|| Domain::try_from("blobfolio.com")),

	Bench::new("adbyss_psl::Domain::try_from::<str>(www.blobfolio.com)")
		.run(|| Domain::try_from("www.blobfolio.com")),

	Bench::new("adbyss_psl::Domain::try_from::<str>(食狮.com.cn)")
		.run(|| Domain::try_from("食狮.com.cn")),

	Bench::new("adbyss_psl::Domain::try_from::<str>(xn--85x722f.com.cn)")
		.run(|| Domain::try_from("xn--85x722f.com.cn")),

	Bench::spacer(),

	Bench::new("adbyss_psl::Domain::try_from::<String>(blobfolio.com)")
		.run_seeded("blobfolio.com".to_owned(), |s| Domain::try_from(s)),

	Bench::new("adbyss_psl::Domain::try_from::<String>(www.blobfolio.com)")
		.run_seeded("www.blobfolio.com".to_owned(), |s| Domain::try_from(s)),

	Bench::new("adbyss_psl::Domain::try_from::<String>(食狮.com.cn)")
		.run_seeded("食狮.com.cn".to_owned(), |s| Domain::try_from(s)),

	Bench::new("adbyss_psl::Domain::try_from::<String>(xn--85x722f.com.cn)")
		.run_seeded("xn--85x722f.com.cn".to_owned(), |s| Domain::try_from(s)),
);
