/*!
# Benchmark: `adbyss_psl::Domain::email`
*/

use brunch::{
	Bench,
	benches,
};
use adbyss_psl::Domain;

benches!(
	Bench::new("adbyss_psl::Domain::email(josh@blobfolio.com)")
		.run(|| Domain::email("josh@blobfolio.com")),

	Bench::new("adbyss_psl::Domain::email(JOSH@BLOBFOLIO.COM)")
		.run(|| Domain::email("JOSH@BLOBFOLIO.COM")),

	Bench::new("adbyss_psl::Domain::email(princess.peach@cat♥.com)")
		.run(|| Domain::email("princess.peach@cat♥.com")),

	Bench::spacer(),

	Bench::new("adbyss_psl::Domain::email_parts(josh@blobfolio.com)")
		.run(|| Domain::email_parts("josh@blobfolio.com")),

	Bench::new("adbyss_psl::Domain::email_parts(JOSH@BLOBFOLIO.COM)")
		.run(|| Domain::email_parts("JOSH@BLOBFOLIO.COM")),

	Bench::new("adbyss_psl::Domain::email_parts(princess.peach@cat♥.com)")
		.run(|| Domain::email_parts("princess.peach@cat♥.com")),
);
