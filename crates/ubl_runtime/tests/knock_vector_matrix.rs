use serde_json::{json, Value};
use ubl_runtime::knock::{knock, KnockError, MAX_ARRAY_LEN, MAX_DEPTH};

#[derive(Clone, Copy)]
enum Expect {
    Ok,
    Err(&'static str),
}

fn code_of(err: &KnockError) -> String {
    err.to_string()
        .split(':')
        .next()
        .unwrap_or("KNOCK-000")
        .to_string()
}

fn json_bytes(v: Value) -> Vec<u8> {
    serde_json::to_vec(&v).expect("serialize vector")
}

fn deep_payload(depth: usize) -> Value {
    let mut node = json!({"leaf": 1});
    for _ in 0..depth {
        node = json!({"next": node});
    }
    json!({
        "@type": "ubl/vector.depth",
        "@world": "a/test/t/vector",
        "payload": node
    })
}

fn oversized_array_payload() -> Value {
    let arr: Vec<Value> = (0..=MAX_ARRAY_LEN).map(|_| json!(1)).collect();
    json!({
        "@type": "ubl/vector.array",
        "@world": "a/test/t/vector",
        "arr": arr
    })
}

#[test]
fn knock_vector_matrix_should_cover_wide_surface() {
    let cases: Vec<(&str, Vec<u8>, Expect)> = vec![
        (
            "valid_minimal",
            json_bytes(json!({
                "@type": "ubl/user",
                "@world": "a/app/t/main",
                "email": "alice@example.com"
            })),
            Expect::Ok,
        ),
        (
            "valid_unc1_dec_atom",
            json_bytes(json!({
                "@type": "ubl/payment",
                "@world": "a/app/t/main",
                "amount": {"@num": "dec/1", "m": "12345", "s": 2}
            })),
            Expect::Ok,
        ),
        (
            "valid_unc1_rat_atom",
            json_bytes(json!({
                "@type": "ubl/ratio",
                "@world": "a/app/t/main",
                "ratio": {"@num": "rat/1", "p": "1", "q": "3"}
            })),
            Expect::Ok,
        ),
        (
            "missing_type",
            json_bytes(json!({"@world": "a/app/t/main"})),
            Expect::Err("KNOCK-006"),
        ),
        (
            "missing_world",
            json_bytes(json!({"@type": "ubl/user"})),
            Expect::Err("KNOCK-006"),
        ),
        (
            "not_object",
            json_bytes(json!([1, 2, 3])),
            Expect::Err("KNOCK-007"),
        ),
        (
            "raw_float_literal",
            json_bytes(json!({
                "@type": "ubl/payment",
                "@world": "a/app/t/main",
                "amount": 12.34
            })),
            Expect::Err("KNOCK-008"),
        ),
        (
            "malformed_num_missing_fields",
            json_bytes(json!({
                "@type": "ubl/payment",
                "@world": "a/app/t/main",
                "amount": {"@num": "dec/1", "m": "100"}
            })),
            Expect::Err("KNOCK-009"),
        ),
        (
            "malformed_num_unknown_tag",
            json_bytes(json!({
                "@type": "ubl/payment",
                "@world": "a/app/t/main",
                "amount": {"@num": "foo/1", "x": "1"}
            })),
            Expect::Err("KNOCK-009"),
        ),
        (
            "malformed_num_zero_denominator",
            json_bytes(json!({
                "@type": "ubl/payment",
                "@world": "a/app/t/main",
                "amount": {"@num": "rat/1", "p": "1", "q": "0"}
            })),
            Expect::Err("KNOCK-009"),
        ),
        (
            "duplicate_key_exact_raw",
            br#"{"@type":"ubl/user","@world":"a/app/t/main","x":1,"x":2}"#.to_vec(),
            Expect::Err("KNOCK-004"),
        ),
        (
            "duplicate_key_after_nfc_normalization",
            br#"{"@type":"ubl/user","@world":"a/app/t/main","Cafe\u0301":"A","Caf\u00e9":"B"}"#
                .to_vec(),
            Expect::Err("KNOCK-004"),
        ),
        (
            "input_normalization_control_char",
            br#"{"@type":"ubl/user","@world":"a/app/t/main","name":"bad\u0001char"}"#.to_vec(),
            Expect::Err("KNOCK-011"),
        ),
        (
            "array_too_long",
            json_bytes(oversized_array_payload()),
            Expect::Err("KNOCK-003"),
        ),
        (
            "depth_exceeded",
            json_bytes(deep_payload(MAX_DEPTH + 4)),
            Expect::Err("KNOCK-002"),
        ),
        (
            "invalid_utf8_raw",
            vec![0xff, 0xfe, 0x00],
            Expect::Err("KNOCK-005"),
        ),
    ];

    for (name, bytes, expect) in cases {
        let result = knock(&bytes);
        match expect {
            Expect::Ok => {
                assert!(result.is_ok(), "case {name} should accept, got {result:?}");
            }
            Expect::Err(expected_code) => {
                let err =
                    result.expect_err(&format!("case {name} should reject with {expected_code}"));
                let code = code_of(&err);
                assert_eq!(
                    code, expected_code,
                    "case {name} wrong code: expected {expected_code}, got {code}, err={err}"
                );
            }
        }
    }
}

#[test]
fn knock_contract_malformed_json_should_not_be_classified_as_utf8_error() {
    let malformed_but_utf8 = br#"{"@type":"ubl/user","@world":"a/app/t/main",}"#;
    let err = knock(malformed_but_utf8).expect_err("malformed json must reject");
    assert!(
        !matches!(err, KnockError::InvalidUtf8),
        "malformed JSON (valid UTF-8) should not map to InvalidUtf8; got: {err}"
    );
}

#[test]
fn knock_contract_u64_overflow_should_be_rejected_before_nrf_stage() {
    let payload = json!({
        "@type": "ubl/user",
        "@world": "a/app/t/main",
        "u": u64::MAX
    });
    let err = knock(&json_bytes(payload))
        .expect_err("u64::MAX should be rejected at KNOCK to avoid NRF mismatch later");
    let code = code_of(&err);
    assert!(
        ["KNOCK-008", "KNOCK-010", "KNOCK-011"].contains(&code.as_str()),
        "unexpected code for u64 overflow guard: {code} ({err})"
    );
}
