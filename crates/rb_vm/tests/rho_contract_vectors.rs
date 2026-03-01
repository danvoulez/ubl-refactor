use rb_vm::canon::{CanonProvider, RhoCanon};
use serde_json::{json, Map, Value};

#[test]
fn rho_vector_matrix_basic_behavior() {
    let cases: Vec<(&str, Value)> = vec![
        ("empty", json!({})),
        ("booleans", json!({"t": true, "f": false})),
        ("int_boundaries", json!({"min": i64::MIN, "max": i64::MAX})),
        ("null_stripping", json!({"a": 1, "b": null})),
        ("nested", json!({"z": {"b": 2, "a": 1}})),
        ("array", json!([{"b": 2, "a": 1}, null, 3])),
    ];

    for (name, input) in cases {
        let once = RhoCanon.canon(input.clone());
        let twice = RhoCanon.canon(once.clone());
        assert_eq!(once, twice, "rho must be idempotent for case {name}");
    }
}

#[test]
fn rho_contract_collision_after_nfc_normalization_must_not_silently_overwrite() {
    let mut map = Map::new();
    map.insert("Cafe\u{0301}".to_string(), json!(1));
    map.insert("Caf\u{00e9}".to_string(), json!(2));

    let out = RhoCanon.canon(Value::Object(map));
    let obj = out.as_object().expect("must be object");

    assert_eq!(
        obj.len(),
        2,
        "ρ must not silently collapse keys that collide after NFC normalization"
    );
}

#[test]
fn rho_contract_raw_float_must_not_be_poisoned_into_string() {
    let out = RhoCanon.canon(json!({"amount": 12.34}));
    let amount = &out["amount"];

    assert!(
        !amount
            .as_str()
            .map(|s| s.starts_with("__FLOAT_REJECTED:"))
            .unwrap_or(false),
        "ρ should reject/flag float canonization, not encode poison strings"
    );
}

#[test]
fn rho_contract_control_chars_should_be_rejected_in_validation() {
    let bad = json!({"s": "line\nbreak"});
    let errors = RhoCanon::validate(&bad)
        .expect_err("control characters should be reported as validation errors");
    assert!(
        errors.iter().any(|e| e.contains("control character")),
        "expected control char validation error, got: {errors:?}"
    );
}
