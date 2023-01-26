/*!
# Adbyss: Internet Check
*/

use crate::AdbyssError;
use std::{
	thread::sleep,
	time::Duration,
};
use ureq::AgentBuilder;



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
	let agent = AgentBuilder::new()
		.timeout(Duration::from_secs(15))
		.user_agent("Mozilla/5.0")
		.max_idle_connections(0)
		.build();

	let mut tries: u8 = 0;
	loop {
		// Are you there?
		if matches!(agent.head("https://github.com/").call().map(|r| r.status()), Ok(200_u16)) {
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
