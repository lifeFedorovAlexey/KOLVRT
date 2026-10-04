//! Offline research consistency checks. Passing does not establish historical truth.
#![forbid(unsafe_code)]

pub mod cost_l;
pub mod database;
pub mod documents;
pub mod exceptions;
pub mod knowledge;
pub mod report;
pub mod security_artifacts;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{fmt, fs, path::Path};

// Minimum evidence corpus required by the Phase 0.1 research scope.
pub const MINIMUM_RESEARCH_CASES: usize = 30;

pub type CheckResult<T> = Result<T, String>;

pub fn read(path: &Path) -> CheckResult<String> {
    fs::read_to_string(path)
        .map(|s| s.replace("\r\n", "\n"))
        .map_err(|e| format!("{}: {e}", path.display()))
}

// serde_json::Value normally accepts the last occurrence of an object key.
// Research evidence must not silently lose an earlier, conflicting value.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("strict JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = Vec::new();
                while let Some(Strict(v)) = a.next_element()? {
                    values.push(v);
                }
                Ok(Strict(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if values.contains_key(&k) {
                        return Err(de::Error::custom(format!("duplicate JSON key: {k}")));
                    }
                    let Strict(v) = a.next_value()?;
                    values.insert(k, v);
                }
                Ok(Strict(Value::Object(values)))
            }
        }
        d.deserialize_any(JsonVisitor)
    }
}

pub fn parse_json(text: &str) -> CheckResult<Value> {
    serde_json::from_str::<Strict>(text)
        .map(|v| v.0)
        .map_err(|e| e.to_string())
}
pub fn read_json(path: &Path) -> CheckResult<Value> {
    parse_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn hash(text: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(text.replace("\r\n", "\n").as_bytes())
    )
}
pub fn write_json(path: &Path, value: &Value) -> CheckResult<()> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())? + "\n";
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn finish(errors: Vec<String>) -> CheckResult<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
