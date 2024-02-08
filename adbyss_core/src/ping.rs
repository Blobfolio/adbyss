/*!
# Adbyss: Internet Check
*/

use crate::AdbyssError;
use std::{
	thread::sleep,
	time::Duration,
};



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
pub fn check_internet() -> Result<(), AdbyssError> {
	let mut tries: u8 = 0;
	loop {
		// Are you there?
		let res = minreq::head("https://github.com/")
			.with_header("user-agent", "Mozilla/5.0")
			.with_timeout(15)
			.send();

		if res.map_or(false, |r| r.status_code == 200) {
			return Ok(());
		}

		// Out of tries?
		if tries == 9 {
			return Err(AdbyssError::NoInternet);
		}

		// Wait and try again.
		tries += 1;
		sleep(Duration::from_secs(10));
	}

}
