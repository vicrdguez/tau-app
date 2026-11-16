mod common;

use common::fixture;
use serde_json::json;
use tau_client::wire::{Outcome, Paired, ProjectView, RefusalCode, Token, Unknown, parse_reply};

fn body(v: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

#[test]
fn the_body_decides_not_the_status() {
    let refused = body(&json!({"refused": {"code": "conflict", "message": "m"}}));
    assert!(
        matches!(parse_reply(200, Some("1"), &refused), Outcome::Refused(r) if r.code == RefusalCode::Conflict)
    );
    let done = body(&json!({"result": {"ok": true}}));
    assert_eq!(
        parse_reply(409, Some("1"), &done),
        Outcome::Done(json!({"ok": true}))
    );
    let page = b"<html><body>409 Conflict</body></html>";
    assert_eq!(
        parse_reply(409, None, page),
        Outcome::Unknown(Unknown::Unreadable { status: 409 })
    );
    assert_eq!(
        parse_reply(408, None, b""),
        Outcome::Unknown(Unknown::Unreadable { status: 408 })
    );
}

#[test]
fn other_json_is_unknown() {
    for v in [
        json!({}),
        json!({"error": "x"}),
        json!([1]),
        json!({"refused": "nope"}),
        json!({"refused": {"message": "no code"}}),
    ] {
        assert_eq!(
            parse_reply(400, Some("1"), &body(&v)),
            Outcome::Unknown(Unknown::Unreadable { status: 400 }),
            "{v}"
        );
    }
}

#[test]
fn the_unknown_envelope_is_unknown() {
    let reply = &fixture()["refusals"]["unknown (storage)"]["reply"];
    assert_eq!(
        parse_reply(500, Some("1"), &body(reply)),
        Outcome::Unknown(Unknown::Server {
            code: "storage_failed".into(),
            message: "SQLITE_FULL: database or disk is full".into()
        })
    );
}

#[test]
fn another_protocol_is_unknown_and_named() {
    let done = body(&json!({"result": {}}));
    assert_eq!(
        parse_reply(200, Some("2"), &done),
        Outcome::Unknown(Unknown::OtherProtocol("2".into()))
    );
}

#[test]
fn every_fixture_refusal_reads_with_its_fields() {
    let refusals = fixture()["refusals"].as_object().unwrap();
    let mut seen = 0;
    for (name, case) in refusals {
        let Outcome::Refused(r) = parse_reply(409, Some("1"), &body(&case["reply"])) else {
            continue;
        };
        seen += 1;
        assert!(
            !matches!(r.code, RefusalCode::Other(_)),
            "{name}: {:?}",
            r.code
        );
        assert_eq!(r.code.as_str(), case["reply"]["refused"]["code"], "{name}");
        assert!(!r.message.is_empty(), "{name}");
        match r.code {
            RefusalCode::OptionsInvalid | RefusalCode::BadOptions | RefusalCode::BadRequest => {
                assert!(!r.errors().is_empty(), "{name} errors")
            }
            RefusalCode::Conflict if name.starts_with("conflict (answer") => {
                assert_eq!(r.decision().unwrap().by.unwrap().as_str(), "phone")
            }
            RefusalCode::Conflict => {
                assert_eq!(r.project().unwrap().project.as_str(), "render-core")
            }
            RefusalCode::NotOpen => assert_eq!(r.open().unwrap().id.as_str(), "w1:3:design#3"),
            _ => {}
        }
    }
    assert_eq!(
        seen,
        refusals.len() - 1,
        "all but the unknown envelope are refusals"
    );
    let errors = parse_reply(
        409,
        Some("1"),
        &body(
            &fixture()["refusals"]["options_invalid (create w1 in render-core, a model the server no longer has)"]
                ["reply"],
        ),
    );
    let Outcome::Refused(r) = errors else {
        panic!()
    };
    assert_eq!(r.errors()[0].path, "/reviewer");
    let discovery = parse_reply(
        404,
        Some("1"),
        &body(&fixture()["server"]["discovery"]["reply"]),
    );
    let Outcome::Refused(r) = discovery else {
        panic!()
    };
    assert_eq!(
        (r.code.clone(), r.calls().len()),
        (RefusalCode::UnknownCall, 20)
    );
}

#[test]
fn an_unknown_code_is_a_refusal() {
    let reply = body(&json!({"refused": {"code": "melted", "message": "The server melted."}}));
    let Outcome::Refused(r) = parse_reply(409, Some("1"), &reply) else {
        panic!()
    };
    assert_eq!(
        (r.code, r.message.as_str()),
        (RefusalCode::Other("melted".into()), "The server melted.")
    );
}

#[test]
fn unknown_fields_are_ignored() {
    let v =
        json!({"project": "p", "workdir": "/w", "extensions": [], "missing": [], "colour": "teal"});
    let p: ProjectView = serde_json::from_value(v).unwrap();
    assert_eq!(p.project.as_str(), "p");
}

#[test]
fn the_token_never_prints() {
    let paired: Paired =
        serde_json::from_value(fixture()["pair"]["reply"]["result"].clone()).unwrap();
    let secret = paired.token.expose().to_owned();
    assert!(!format!("{paired:?}").contains(&secret[11..]));
    assert!(!format!("{:?}", Token::new("tau.x.abc")).contains("abc"));
}
