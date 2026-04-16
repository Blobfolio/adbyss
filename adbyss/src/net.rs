/*!
# Adbyss: Networking.
*/

use crate::AdbyssError;
use reqwest::{
	blocking::{
		Client,
		ClientBuilder,
	},
	StatusCode,
};
use std::{
	sync::OnceLock,
	thread::sleep,
	time::Duration,
};

/// # HTTP Client.
static CLIENT: OnceLock<Option<Client>> = OnceLock::new();



/// # Check Internet.
///
/// This method attempts to check for an internet connection by trying to reach
/// Github (which is serving one of the lists Adbyss needs anyway). It will
/// give it ten tries, with ten seconds in between each try, returning an
/// error if nothing has been reached after that.
///
/// ## Errors
///
/// If the site can't be reached, an error will be returned.
pub(super) fn check_internet() -> Result<(), AdbyssError> {
	let mut tries: u8 = 0;
	loop {
		// Are you there?
		let res = client()
			.ok_or(AdbyssError::NoInternet)?
			.head("https://github.com/")
			.send();

		if res.is_ok_and(|r| matches!(r.status(), StatusCode::OK)) { return Ok(()); }

		// Out of tries?
		if tries == 9 { return Err(AdbyssError::NoInternet); }

		// Wait and try again.
		tries += 1;
		sleep(Duration::from_secs(10));
	}
}

#[must_use]
/// # Initialize HTTP Client.
pub(super) fn client() -> Option<&'static Client> {
	CLIENT.get_or_init(||
		ClientBuilder::new()
			.user_agent("Mozilla/5.0")
			.gzip(true)
			.build()
			.ok()
	)
	.as_ref()
}
