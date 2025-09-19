/*!
# Adbyss: Serde Extensions.
*/

use serde_core::{
	de,
	Deserialize,
	Serialize,
	Serializer,
};
use std::fmt;
use super::Domain;



impl Serialize for Domain {
	#[inline]
	/// # Serialize `Domain`.
	///
	/// Use the optional `serde` crate feature to enable serialization support
	/// for [`Domain`]s.
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer { serializer.serialize_str(&self.host) }
}

impl<'de> Deserialize<'de> for Domain {
	/// # Deserialize `Domain`.
	///
	/// Use the optional `serde` crate feature to enable serialization support
	/// for [`Domain`]s.
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where D: de::Deserializer<'de> {
		/// # Visitor Instance.
		struct DomainVisitor;

		impl de::Visitor<'_> for DomainVisitor {
			type Value = Domain;
			fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
				f.write_str("domain string")
			}

			fn visit_str<S>(self, src: &str) -> Result<Domain, S>
			where S: de::Error {
				Domain::new(src)
					.ok_or_else(|| de::Error::custom("invalid domain"))
			}

			fn visit_bytes<S>(self, src: &[u8]) -> Result<Domain, S>
			where S: de::Error {
				std::str::from_utf8(src)
					.ok()
					.and_then(Domain::new)
					.ok_or_else(|| de::Error::custom("invalid domain"))
			}
		}

		deserializer.deserialize_str(DomainVisitor)
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// # Serde tests.
	fn t_serde() {
		let dom1: Domain = Domain::new("serialize.domain.com")
			.expect("Domain failed.");

		// Serialize it.
		let serial: String = serde_json::to_string(&dom1)
			.expect("Serialize failed.");
		assert_eq!(serial, "\"serialize.domain.com\"");

		// Deserialize it.
		let dom2: Domain = serde_json::from_str(&serial).expect("Deserialize failed.");
		assert_eq!(dom1, dom2);
	}
}
