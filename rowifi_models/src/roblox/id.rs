use bytes::BytesMut;
use serde::{
    de::{Deserializer, Error as DeError, Unexpected, Visitor},
    Deserialize, Serialize,
};
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AssetId(pub u64);

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct GroupId(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RoleId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct UserId(pub u64);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UniverseId(pub u64);

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for GroupId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for RoleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Display for UniverseId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl From<u64> for UserId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<u64> for UniverseId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl ToSql for UserId {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        #[allow(clippy::cast_possible_wrap)]
        i64::to_sql(&(self.0 as i64), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for UserId {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        let id = i64::from_sql(ty, raw)?;
        #[allow(clippy::cast_sign_loss)]
        Ok(Self(id as u64))
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as FromSql>::accepts(ty)
    }
}

impl ToSql for GroupId {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        #[allow(clippy::cast_possible_wrap)]
        i64::to_sql(&(self.0 as i64), ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for GroupId {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        let id = i64::from_sql(ty, raw)?;
        #[allow(clippy::cast_sign_loss)]
        Ok(Self(id as u64))
    }

    fn accepts(ty: &Type) -> bool {
        <i64 as FromSql>::accepts(ty)
    }
}

struct IdVisitor<V> {
    _p: PhantomData<V>,
}

impl<'de, V> Visitor<'de> for IdVisitor<V>
where
    V: From<u64>,
{
    type Value = V;

    fn expecting(&self, f: &mut Formatter) -> FmtResult {
        f.write_str("a roblox id")
    }

    fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Self::Value::from(v))
    }

    fn visit_i64<E: DeError>(self, v: i64) -> Result<Self::Value, E> {
        #[allow(clippy::cast_sign_loss)]
        let val = v as u64;
        self.visit_u64(val)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(IdVisitor { _p: PhantomData })
    }

    fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
        let value = v.parse().map_err(|_| {
            let unexpected = Unexpected::Str(v);
            DeError::invalid_value(unexpected, &"a u64 string")
        })?;

        self.visit_u64(value)
    }
}

impl<'de> Deserialize<'de> for UserId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(IdVisitor { _p: PhantomData })
    }
}

impl<'de> Deserialize<'de> for UniverseId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(IdVisitor { _p: PhantomData })
    }
}
