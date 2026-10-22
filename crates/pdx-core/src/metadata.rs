use crate::{Diagnostic, Result};
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const METADATA_BYTES: usize = 1_048_576;

pub fn parse_json(text: &str) -> Result<Value> {
    if text.len() > METADATA_BYTES {
        return Err(Diagnostic::input("parse_budget", "JSON exceeds 1 MiB"));
    }
    serde_json::from_str::<JsonValue>(text)
        .map(|v| v.0)
        .map_err(|e| Diagnostic::input("invalid_json", e.to_string()))
}

/// JSON-compatible YAML values; the key visitor refuses numeric/bool key coercion.
struct JsonValue(Value);
struct StringKey(String);
impl<'de> Deserialize<'de> for StringKey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct KeyVisitor;
        impl Visitor<'_> for KeyVisitor {
            type Value = StringKey;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string mapping key")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(StringKey(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(StringKey(v))
            }
        }
        d.deserialize_any(KeyVisitor)
    }
}
impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = JsonValue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON-compatible value")
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                Number::from_f64(v)
                    .map(|n| JsonValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(JsonValue(v.into()))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(JsonValue(v)) = a.next_element()? {
                    values.push(v);
                }
                Ok(JsonValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some((StringKey(k), JsonValue(v))) = a.next_entry()? {
                    if values.insert(k, v).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(JsonValue(Value::Object(values)))
            }
        }
        d.deserialize_any(JsonVisitor)
    }
}

pub fn parse_metadata(text: &str) -> Result<Value> {
    if text.len() > METADATA_BYTES {
        return Err(Diagnostic::input("parse_budget", "metadata exceeds 1 MiB"));
    }
    let options = serde_saphyr::options! {
        budget: serde_saphyr::budget! {
            max_documents: 1, max_depth: 32, max_nodes: 10000, max_events: 40000,
            max_aliases: 0, max_anchors: 0, max_merge_keys: 0,
            max_total_scalar_bytes: METADATA_BYTES, flow_nesting_limit: 32,
        },
        strict_booleans: true,
        reject_unsupported_tags: true,
        with_snippet: false,
    };
    let JsonValue(value) = serde_saphyr::from_str_with_options(text, options)
        .map_err(|e| Diagnostic::input("invalid_metadata", e.to_string()))?;
    if !value.is_object() {
        return Err(Diagnostic::input(
            "invalid_metadata",
            "metadata must be a mapping",
        ));
    }
    Ok(value)
}

pub fn merge_metadata(base: &mut Value, overlay: Value) {
    if let (Some(base), Value::Object(overlay)) = (base.as_object_mut(), &overlay) {
        for (key, value) in overlay {
            merge_metadata(
                base.entry(key.clone()).or_insert(Value::Null),
                value.clone(),
            );
        }
    } else {
        *base = overlay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn recursive_merge_and_array_replacement() {
        let mut a = json!({"a":{"x":1,"y":2},"b":[1,2]});
        merge_metadata(&mut a, json!({"a":{"x":3},"b":[4]}));
        assert_eq!(a, json!({"a":{"x":3,"y":2},"b":[4]}));
    }
    #[test]
    fn strict_yaml() {
        for text in [
            "a: 1\na: 2",
            "a: 1\n---\na: 2",
            "1: hello",
            "true: hi",
            "a: !include x",
            "a: !evil x",
            "a: &a hi\nb: *a",
            "[]",
            "a: .inf",
        ] {
            assert!(parse_metadata(text).is_err(), "{text}");
        }
        assert_eq!(
            parse_metadata("a: 1\r\nb: 日本語\r\n").unwrap(),
            json!({"a":1,"b":"日本語"})
        );
    }
}
