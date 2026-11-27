//! The JSON schemas in `contracts/` accept what they should and refuse what they must.
//! The extraction schema is a security boundary: a quarantine model's output is valid only if it
//! contains nothing but typed fields from closed lists.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use std::path::PathBuf;

use serde_json::Value;

fn contracts() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts")
}

fn load(rel: &str) -> Value {
    let text =
        std::fs::read_to_string(contracts().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn check(name: &str) {
    let validator = jsonschema::validator_for(&load(&format!("{name}.schema.json")))
        .unwrap_or_else(|e| panic!("{name} schema: {e}"));
    let valid = load(&format!("fixtures/{name}/valid.json"));
    assert!(
        validator.is_valid(&valid),
        "{name}: the valid fixture was rejected"
    );
    let invalid = load(&format!("fixtures/{name}/invalid.json"));
    for (i, case) in invalid
        .as_array()
        .expect("array of invalid cases")
        .iter()
        .enumerate()
    {
        assert!(
            !validator.is_valid(case),
            "{name}: invalid case {i} was accepted: {case}"
        );
    }
}

#[test]
fn delta_schema() {
    check("delta");
}

#[test]
fn extraction_schema() {
    check("extraction");
}

#[test]
fn events_schema() {
    check("events");
}

#[test]
fn relay_job_schema() {
    check("relay-job");
    let validator = jsonschema::validator_for(&load("relay-job.schema.json")).unwrap();
    assert!(validator.is_valid(&load("fixtures/relay-job/valid-job.json")));
}

#[test]
fn every_schema_file_has_a_fixture_set() {
    for entry in std::fs::read_dir(contracts()).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(base) = name.strip_suffix(".schema.json") {
            assert!(
                contracts()
                    .join(format!("fixtures/{base}/valid.json"))
                    .exists(),
                "{base}: no valid fixture"
            );
            assert!(
                contracts()
                    .join(format!("fixtures/{base}/invalid.json"))
                    .exists(),
                "{base}: no invalid fixtures"
            );
        }
    }
}
