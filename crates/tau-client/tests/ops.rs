use serde_json::{Value, json};
use tau_client::ops::{OpError, apply};

fn applied(mut value: Value, ops: Value) -> Result<Value, OpError> {
    apply(&mut value, ops.as_array().unwrap())?;
    Ok(value)
}

#[test]
fn replace_drops_the_old_value() {
    let v = applied(json!({"a": 1, "b": 2}), json!([["r", {"c": 3}]])).unwrap();
    assert_eq!(v, json!({"c": 3}));
}

#[test]
fn set_appends_at_the_end_and_refuses_beyond() {
    let v = applied(
        json!({"xs": [1, 2]}),
        json!([["s", ["xs", 2], 3], ["s", ["xs", 0], 0], ["s", ["k"], {"n": 1}]]),
    );
    assert_eq!(v.unwrap(), json!({"xs": [0, 2, 3], "k": {"n": 1}}));
    assert!(matches!(
        applied(json!({"xs": [1]}), json!([["s", ["xs", 3], 9]])),
        Err(OpError::UnsafePath(_) | OpError::Path(_))
    ));
}

#[test]
fn a_path_through_nothing_is_refused() {
    assert!(matches!(
        applied(json!({}), json!([["s", ["docs", "pi.live", "run"], 1]])),
        Err(OpError::Path(_))
    ));
    assert!(matches!(
        applied(json!({"a": "text"}), json!([["s", ["a", "b"], 1]])),
        Err(OpError::Path(_))
    ));
    assert!(applied(json!({"xs": []}), json!([["s", ["xs", "k"], 1]])).is_err());
}

#[test]
fn delete_removes_array_items_and_keys() {
    let v = applied(
        json!({"xs": [1, 2, 3], "k": 1, "j": 2}),
        json!([["d", ["xs", 1]], ["d", ["k"]]]),
    )
    .unwrap();
    assert_eq!(v, json!({"xs": [1, 3], "j": 2}));
    assert!(applied(json!({"xs": [1]}), json!([["d", ["xs", 1]]])).is_err());
}

#[test]
fn append_and_trim_need_a_string() {
    let v = applied(
        json!({"t": "ab"}),
        json!([["a", ["t"], "cd"], ["t", ["t"], 1]]),
    )
    .unwrap();
    assert_eq!(v, json!({"t": "bcd"}));
    assert!(matches!(
        applied(json!({"t": 1}), json!([["a", ["t"], "x"]])),
        Err(OpError::Path(_))
    ));
    assert!(matches!(
        applied(json!({}), json!([["a", ["t"], "x"]])),
        Err(OpError::Path(_))
    ));
    assert!(matches!(
        applied(json!({"t": ["x"]}), json!([["t", ["t"], 1]])),
        Err(OpError::Path(_))
    ));
}

#[test]
fn trim_counts_utf16_units() {
    // ✔ is one UTF-16 unit and three UTF-8 bytes; 🙂 is two units and four bytes.
    let v = applied(json!({"t": "✔🙂ok"}), json!([["t", ["t"], 3]])).unwrap();
    assert_eq!(v, json!({"t": "ok"}));
    let v = applied(json!({"t": "✔ok"}), json!([["t", ["t"], 1]])).unwrap();
    assert_eq!(v, json!({"t": "ok"}));
    assert!(applied(json!({"t": "ab"}), json!([["t", ["t"], 5]])).unwrap() == json!({"t": ""}));
}

#[test]
fn splice_removes_then_inserts() {
    let v = applied(
        json!({"xs": [1, 2, 3, 4]}),
        json!([["p", ["xs"], 1, 2, ["a", "b", "c"]]]),
    )
    .unwrap();
    assert_eq!(v, json!({"xs": [1, "a", "b", "c", 4]}));
    let v = applied(json!({"xs": [1]}), json!([["p", ["xs"], 1, 0, [2]]])).unwrap();
    assert_eq!(v, json!({"xs": [1, 2]}));
    assert!(applied(json!({"xs": [1]}), json!([["p", ["xs"], 2, 0, [2]]])).is_err());
}

#[test]
fn splice_and_reorder_reach_the_root() {
    let v = applied(
        json!([1, 2]),
        json!([
            ["p", [], 0, 1, []],
            ["p", [], 1, 0, [3, 4]],
            ["m", [], [2, 0, 1]]
        ]),
    )
    .unwrap();
    assert_eq!(v, json!([4, 2, 3]));
}

#[test]
fn reorder_takes_new_from_old_by_order() {
    let v = applied(
        json!({"xs": ["a", "b", "c"]}),
        json!([["m", ["xs"], [2, 0, 1]]]),
    )
    .unwrap();
    assert_eq!(v, json!({"xs": ["c", "a", "b"]}));
}

#[test]
fn reorder_refuses_what_is_not_a_permutation() {
    assert!(applied(json!({"xs": ["a", "b"]}), json!([["m", ["xs"], [0, 0]]])).is_err());
    assert!(applied(json!({"xs": ["a", "b"]}), json!([["m", ["xs"], [0]]])).is_err());
    assert!(applied(json!({"xs": ["a", "b"]}), json!([["m", ["xs"], [0, 2]]])).is_err());
}

#[test]
fn prototype_segments_are_refused_anywhere() {
    for seg in ["__proto__", "constructor", "prototype"] {
        let deep = json!([["s", ["a", seg, "b"], 1]]);
        assert!(
            matches!(applied(json!({"a": {}}), deep), Err(OpError::UnsafePath(_))),
            "{seg} inside"
        );
        let last = json!([["s", ["a", seg], 1]]);
        assert!(
            matches!(applied(json!({"a": {}}), last), Err(OpError::UnsafePath(_))),
            "{seg} last"
        );
        let splice = json!([["p", [seg], 0, 0, []]]);
        assert!(
            matches!(applied(json!({}), splice), Err(OpError::UnsafePath(_))),
            "{seg} in p"
        );
    }
}

#[test]
fn only_r_p_m_reach_the_root() {
    for op in [
        json!(["s", [], 1]),
        json!(["d", []]),
        json!(["a", [], "x"]),
        json!(["t", [], 1]),
    ] {
        assert!(applied(json!("text"), json!([op.clone()])).is_err(), "{op}");
    }
}

#[test]
fn unknown_or_short_ops_are_refused() {
    for op in [
        json!(["x", ["a"], 1]),
        json!(["s", 1]),
        json!(["a", "text"]),
        json!(["p", 0, 0, []]),
        json!("s"),
        json!([]),
    ] {
        assert!(
            matches!(
                applied(json!({"a": 1}), json!([op.clone()])),
                Err(OpError::Shape(_))
            ),
            "{op}"
        );
    }
}

#[test]
fn ops_apply_in_order() {
    let v = applied(
        json!({}),
        json!([
            ["s", ["t"], "a"],
            ["a", ["t"], "b"],
            ["s", ["xs"], []],
            ["p", ["xs"], 0, 0, [1]],
            ["d", ["t"]]
        ]),
    );
    assert_eq!(v.unwrap(), json!({"xs": [1]}));
}

#[test]
fn indexes_are_whole_and_non_negative() {
    assert!(applied(json!({"xs": [1, 2]}), json!([["s", ["xs", -1], 0]])).is_err());
    assert!(applied(json!({"xs": [1, 2]}), json!([["s", ["xs", 0.5], 0]])).is_err());
    assert!(applied(json!({"xs": [1, 2]}), json!([["p", ["xs"], -1, 0, []]])).is_err());
}
