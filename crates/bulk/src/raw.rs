//! Bounded duplicate-safe raw JSON, without a floating JSON value conversion.
use crate::{Error, fills::MAX_BODY, precision::decimal};
use serde::{
    Deserialize,
    de::{self, DeserializeOwned, MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{collections::BTreeMap, fmt};

pub(crate) struct Object(BTreeMap<String, Box<RawValue>>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded unique object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some((name, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if name.len() > 64 || values.len() >= 64 || values.insert(name, value).is_some()
                    {
                        return Err(de::Error::custom("ambiguous or oversized object"));
                    }
                }
                Ok(Object(values))
            }
        }
        d.deserialize_map(Fields)
    }
}
impl Object {
    pub(crate) fn get<T: DeserializeOwned>(&self, name: &str) -> Result<T, Error> {
        decode(self.0.get(name).ok_or(Error::Invalid)?.get().as_bytes())
    }
    pub(crate) fn optional<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, Error> {
        self.0
            .get(name)
            .map(|v| decode(v.get().as_bytes()))
            .transpose()
    }
    pub(crate) fn amount(&self, name: &str) -> Result<String, Error> {
        scalar(self.0.get(name).ok_or(Error::Invalid)?)
    }
    pub(crate) fn optional_amount(&self, name: &str) -> Result<Option<String>, Error> {
        self.0.get(name).map(|v| scalar(v)).transpose()
    }
}
pub(crate) fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, Error> {
    if body.is_empty() || body.len() > MAX_BODY {
        return Err(Error::Limit);
    }
    serde_json::from_slice(body).map_err(|_| Error::Invalid)
}
pub(crate) fn scalar(raw: &RawValue) -> Result<String, Error> {
    let text = if raw.get().starts_with('"') {
        serde_json::from_str::<String>(raw.get()).map_err(|_| Error::Invalid)?
    } else {
        raw.get().into()
    };
    decimal(&text)?;
    Ok(text)
}

/// Exact original private JSON, not a public debug/log representation.
#[derive(Clone)]
pub struct RawRow(pub(crate) Box<RawValue>);
impl RawRow {
    /// Original bytes as UTF-8; preserve privately with the trusted request context.
    pub fn body(&self) -> &str {
        self.0.get()
    }
}
impl PartialEq for RawRow {
    fn eq(&self, other: &Self) -> bool {
        self.body() == other.body()
    }
}
impl Eq for RawRow {}
impl fmt::Debug for RawRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawRow([PRIVATE])")
    }
}
