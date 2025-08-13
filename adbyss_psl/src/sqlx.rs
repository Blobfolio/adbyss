/*!
# Adbyss: Sqlx (Mysql) Extensions.
*/

use sqlx::{
	Database,
	Decode,
	Encode,
	encode::IsNull,
	error::BoxDynError,
	MySql,
	Type,
};
use super::Domain;



impl Type<MySql> for Domain
where for<'x> &'x str: Type<MySql> {
	#[inline]
	/// # Database Type For `Domain`.
	///
	/// Use the optional `sqlx-mysql` crate feature to enable Mysql database
	/// support for [`Domain`]s.
	fn type_info() -> <MySql as Database>::TypeInfo { <&str as Type<MySql>>::type_info() }
}

impl<'r> Decode<'r, MySql> for Domain
where for<'x> &'x str: Decode<'x, MySql> + Type<MySql> {
	#[inline]
	/// # Decode `Domain`.
	///
	/// Use the optional `sqlx-mysql` crate feature to enable Mysql database
	/// decoding support for [`Domain`]s.
	///
	/// Note that this expects a string column.
	fn decode(value: <MySql as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let raw = <&str as Decode<MySql>>::decode(value)?;
		let out = Self::try_from(raw).map_err(|_| "invalid domain")?;
		Ok(out)
	}
}

impl<'q> Encode<'q, MySql> for Domain {
	#[inline]
	/// # Encode `Domain`.
	///
	/// Use the optional `sqlx-mysql` crate feature to enable Mysql database
	/// encoding support for [`Domain`]s.
	///
	/// Note that this expects a string column.
	fn encode_by_ref(
		&self,
		buf: &mut <MySql as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, BoxDynError> {
		Encode::<'_, MySql>::encode_by_ref(&self.as_str(), buf)
	}

	#[inline]
	fn produces(&self) -> Option<<MySql as Database>::TypeInfo> {
		<&str as Encode<'_, MySql>>::produces(&self.as_str())
	}

	#[inline]
	fn size_hint(&self) -> usize {
		<&str as Encode<'_, MySql>>::size_hint(&self.as_str())
	}
}
