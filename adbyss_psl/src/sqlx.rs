/*!
# Adbyss: Sqlx (Mysql) Extensions.
*/

use sqlx::{
	Database,
	Decode,
	Encode,
	encode::IsNull,
	error::BoxDynError,
	Type,
};
use super::Domain;



impl<DB> Type<DB> for Domain
where DB: Database, for<'x> &'x str: Type<DB> {
	#[inline]
	/// # Database Type For `Domain`.
	///
	/// Use the optional `sqlx` crate feature to enable Mysql database
	/// support for [`Domain`]s.
	fn type_info() -> <DB as Database>::TypeInfo {
		<&str as Type<DB>>::type_info()
	}

	/// # Compatibility.
	fn compatible(ty: &<DB as Database>::TypeInfo) -> bool {
		<&str as Type<DB>>::compatible(ty)
	}
}

impl<'r, DB> Decode<'r, DB> for Domain
where DB: Database, &'r str: Decode<'r, DB> + Type<DB> {
	#[inline]
	/// # Decode `Domain`.
	///
	/// Use the optional `sqlx` crate feature to enable Mysql database
	/// decoding support for [`Domain`]s.
	///
	/// Note that this expects a string column.
	fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let raw = <&str as Decode<DB>>::decode(value)?;
		let out = Self::try_from(raw).map_err(|_| "invalid domain")?;
		Ok(out)
	}
}

impl<'q, DB> Encode<'q, DB> for Domain
where DB: Database, for<'x> &'x str: Encode<'q, DB> {
	#[inline]
	/// # Encode `Domain`.
	///
	/// Use the optional `sqlx` crate feature to enable Mysql database
	/// encoding support for [`Domain`]s.
	///
	/// Note that this expects a string column.
	fn encode_by_ref(
		&self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, BoxDynError> {
		Encode::<'q, DB>::encode_by_ref(&self.as_str(), buf)
	}

	#[inline]
	fn produces(&self) -> Option<<DB as Database>::TypeInfo> {
		<&str as Encode<'q, DB>>::produces(&self.as_str())
	}

	#[inline]
	fn size_hint(&self) -> usize {
		<&str as Encode<'q, DB>>::size_hint(&self.as_str())
	}
}
