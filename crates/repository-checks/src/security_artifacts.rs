//! Offline artifact validation only; no security scenario is executed here.
use crate::{CheckResult, read_json};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

pub fn domain_validator(root: &Path) -> CheckResult<jsonschema::Validator> {
    let schema = read_json(&root.join("schemas/security-domain.schema.json"))?;
    jsonschema::meta::validate(&schema).map_err(|e| e.to_string())?;
    jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|e| e.to_string())
}
pub fn scenarios(value: &Value) -> CheckResult<()> {
    if value["artifact_kind"] != "future-scenario-specifications"
        || value["execution_status"] != "not-executed"
    {
        return Err("research scenarios cannot claim execution evidence".into());
    }
    let cases = value["scenarios"]
        .as_array()
        .ok_or("missing security scenarios")?;
    if cases.is_empty() || cases.len() > 128 {
        return Err("security scenario count outside bounds".into());
    }
    let mut ids = BTreeSet::new();
    for case in cases {
        for field in ["id", "trigger", "expected"] {
            let s = case[field].as_str().ok_or("missing scenario field")?;
            if s.trim().is_empty() || s.len() > 4096 {
                return Err("invalid scenario field".into());
            }
        }
        if case["execution_status"] != "not-executed" || !ids.insert(case["id"].as_str().unwrap()) {
            return Err("duplicate scenario or unsupported execution claim".into());
        }
    }
    Ok(())
}
pub fn check(root: &Path) -> CheckResult<()> {
    let example = read_json(&root.join("research/fixtures/security-domain-review.json"))?;
    domain_validator(root)?
        .validate(&example)
        .map_err(|e| e.to_string())?;
    if example["evidence_state"] != "proposed" {
        return Err("research domain example must remain proposed".into());
    }
    scenarios(&read_json(
        &root.join("research/fixtures/security-boundaries.json"),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
    }
    #[test]
    fn domain_schema_rejects_missing_scope_unknown_fields_and_unbounded_grants() {
        let validator = domain_validator(root()).unwrap();
        let valid =
            read_json(&root().join("research/fixtures/security-domain-review.json")).unwrap();
        assert!(validator.is_valid(&valid));
        let mut missing = valid.clone();
        missing["grants"][0]
            .as_object_mut()
            .unwrap()
            .remove("effect_scope");
        assert!(!validator.is_valid(&missing));
        let mut unknown = valid.clone();
        unknown["usable_handle"] = json!(42);
        assert!(!validator.is_valid(&unknown));
        let mut unbounded = valid.clone();
        unbounded["grants"] = json!(vec![valid["grants"][0].clone(); 65]);
        assert!(!validator.is_valid(&unbounded));
        let mut hardware = valid;
        hardware["devices"][0]["dma_boundary"] = json!("EL0-is-enough");
        assert!(!validator.is_valid(&hardware));
    }
    #[test]
    fn scenario_specs_cannot_masquerade_as_executed_results() {
        let valid = read_json(&root().join("research/fixtures/security-boundaries.json")).unwrap();
        assert!(scenarios(&valid).is_ok());
        let mut claimed = valid.clone();
        claimed["scenarios"][0]["execution_status"] = json!("passed");
        assert!(scenarios(&claimed).is_err());
        let mut duplicate = valid.clone();
        duplicate["scenarios"][1]["id"] = valid["scenarios"][0]["id"].clone();
        assert!(scenarios(&duplicate).is_err());
    }
}
