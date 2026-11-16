//! Failure modes written before implementation.
//! - A fixture refusal is skipped, mapped without its request, or quotes an extra full stop.
//! - Answer bad_request uses the general heading, or hides field errors.
//! - Bind conflicts lose the stored project; not_open loses the open request number.
//! - Missing extensions are inferred from a server sentence instead of request data.
//! - Unknown codes become unknown outcomes; bad_cursor, idle, event_log produce copy.
//! - Unauthorized implies revocation, or a reworded Host message triggers proxy advice.
//! - Restored setters offer retry; exhausted, old, or restored writes promise automatic sends.
//! - Answer, create, and stop unknowns offer retry or lose their distinct warning.
//! - Settlement, sending, waiting, refused, and done rows use the wrong state words.
//! - An event arm invents an echo, duplicates this device's answer, or names a client key.
//! - Answer echoes lose repeated, other-device, continuation, or send-again context.
//! - Refused option saves hide the server's sentence instead of echoing it.
//! - A lost pair reply claims failure, loses the chosen name, or invents a name.
//! - A transport cut is called proof that no server exists; protocol mismatches are hidden.

use super::*;
use serde_json::{Value, json};
use tau_client::{
    Wall,
    engine::{LinkEvent, PairFailure},
    ids::*,
    outbox::*,
    sse::EndReason,
    streams::StreamKey,
    wire::*,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../design/fixture.json")).unwrap()
}
fn refusal_value(code: RefusalCode, message: &str) -> Refusal {
    Refusal {
        code,
        message: message.into(),
        extra: Default::default(),
    }
}
fn entry(call: WriteCall, state: WriteState) -> OutboxEntry {
    serde_json::from_value(json!({"id":1,"call":call,"body":{"project":"render-core","decision":"w1:3:design#2","workflow":"w1"},"state":state,"created":0,"sent_at":null,"tries":0,"restored":false,"from_start":false})).unwrap()
}
fn record() -> DecisionRecord {
    serde_json::from_value(fixture()["refusals"]["conflict (answer w1:3:design#2 after the phone answered approve)"]["reply"]["refused"]["decision"].clone()).unwrap()
}
fn clean(text: &str) {
    assert!(!text.contains("..") && !text.contains(". ."), "{text}");
}

#[test]
fn every_fixture_refusal_has_exact_copy() {
    let f = fixture();
    let cases = [
        (
            "options_invalid (create w1 in scratch)",
            CallName::Create,
            Some("w1 can't start: its options are not all set or don't fit"),
        ),
        (
            "bad_options (setOptions designer to an unknown model)",
            CallName::SetOptions,
            None,
        ),
        (
            "not_a_workflow",
            CallName::Create,
            Some("websearch is not a workflow."),
        ),
        (
            "not_a_workflow (render-core has no workflow w3)",
            CallName::Create,
            Some("render-core has no workflow w3."),
        ),
        (
            "options_invalid (create w1 in render-core, a model the server no longer has)",
            CallName::Create,
            Some("w1 can't start: its options are not all set or don't fit"),
        ),
        (
            "extension_not_loaded (create w1 while the server lacks w1)",
            CallName::Create,
            Some("The server has no code for w1 now."),
        ),
        (
            "unknown_model (create a bare agent on a model the server lacks)",
            CallName::Create,
            Some("This server has no model anthropic/claude-opus-9."),
        ),
        (
            "unknown_model",
            CallName::SetModel,
            Some("This server has no model anthropic/claude-opus-9."),
        ),
        (
            "busy (continue while a run works)",
            CallName::Continue,
            Some("A run is working; there is nothing to continue."),
        ),
        ("idle", CallName::Input, None),
        ("event_log", CallName::Input, None),
        (
            "conflict (answer w1:3:design#2 after the phone answered approve)",
            CallName::Answer,
            Some("CONFLICT"),
        ),
        (
            "not_open (answer w1:3:design#2 after v3 superseded it)",
            CallName::Answer,
            Some("Not recorded. This request is no longer open."),
        ),
        (
            "stale_basis",
            CallName::Answer,
            Some(
                "Not recorded. The server refused it: Decision w1:3:design#2 asks about another basis.",
            ),
        ),
        (
            "bad_annotation",
            CallName::Answer,
            Some(
                "Not recorded. The server refused it: Decision w1:3:design#2 has a range outside its basis text.",
            ),
        ),
        (
            "unknown_choice",
            CallName::Answer,
            Some(
                "Not recorded. The server refused it: Decision w1:3:design#2 does not offer maybe.",
            ),
        ),
        (
            "extension_not_loaded (answer while the server lacks w1)",
            CallName::Answer,
            Some("Not recorded. The server has no code for w1 now; the decision stays open."),
        ),
        (
            "extension_not_loaded (send again after the fault while the server lacks w1)",
            CallName::Answer,
            Some(
                "Not sent again. The server has no code for w1 now; the answer stays recorded and its work is not started.",
            ),
        ),
        (
            "unknown_conversation",
            CallName::Open,
            Some("render-core has no conversation 99."),
        ),
        (
            "conflict (bind with other fields)",
            CallName::Bind,
            Some("render-core is already bound with other fields:"),
        ),
        (
            "unauthorized (revoked)",
            CallName::Input,
            Some("The server refused this device: Send a valid device token."),
        ),
        (
            "too_large",
            CallName::Input,
            Some("Over 1 MiB · the server refused it · nothing was done"),
        ),
        ("unknown (storage)", CallName::Input, Some("no reply")),
        (
            "bad_request",
            CallName::Input,
            Some("The server did not accept the request: The body does not fit the call."),
        ),
    ];
    assert_eq!(cases.len(), f["refusals"].as_object().unwrap().len());
    for (key, call, expected) in cases {
        let case = &f["refusals"][key];
        assert!(!case.is_null(), "{key}");
        let body = case
            .get("request")
            .cloned()
            .unwrap_or_else(|| json!({"decision":"w1:3:design#2"}));
        let action = if key.contains("send again") {
            AnswerAction::SendAgainAfterFault
        } else {
            AnswerAction::Answer
        };
        let outcome = parse_reply(409, Some("1"), &serde_json::to_vec(&case["reply"]).unwrap());
        if let Outcome::Refused(r) = outcome {
            let copy = refusal(call, &body, &r, action);
            let conflict = format!(
                "Not recorded. phone answered approve at {}.",
                crate::time::clock(Wall(1794674520000))
            );
            let expected = if expected == Some("CONFLICT") {
                Some(conflict.as_str())
            } else {
                expected
            };
            assert_eq!(copy.message.as_deref(), expected, "{key}");
            for text in copy
                .message
                .iter()
                .chain(copy.errors.iter().map(|e| &e.message))
            {
                clean(text);
            }
            assert_eq!(copy.errors, r.errors(), "{key}");
            if call == CallName::Bind {
                assert_eq!(copy.project, r.project());
            }
            if r.code == RefusalCode::NotOpen {
                assert_eq!(copy.open.as_deref(), Some("#3"));
            }
        } else if let Outcome::Unknown(reason) = outcome {
            let row = outbox_row(
                &entry(
                    WriteCall::Input,
                    WriteState::Unknown {
                        reason,
                        next: Next::AtConnection,
                    },
                ),
                Wall(0),
            );
            assert_eq!(Some(row.state.as_str()), expected);
        } else {
            panic!("fixture {key} was not a refusal or unknown");
        }
    }
}

#[test]
fn refusal_edges_keep_server_words_and_details() {
    for code in [
        RefusalCode::UnknownProject,
        RefusalCode::UnknownDecision,
        RefusalCode::NotAWorkflow,
    ] {
        for message in ["Gone.", "Gone"] {
            assert_eq!(
                refusal(
                    CallName::Decision,
                    &json!({}),
                    &refusal_value(code.clone(), message),
                    AnswerAction::Answer
                )
                .message
                .as_deref(),
                Some(message)
            );
        }
    }
    let r = refusal_value(RefusalCode::Other("new_code".into()), "Try elsewhere.");
    assert_eq!(
        refusal(CallName::Input, &json!({}), &r, AnswerAction::Answer)
            .message
            .as_deref(),
        Some("Not done: Try elsewhere.")
    );
    for code in [
        RefusalCode::BadCursor,
        RefusalCode::Idle,
        RefusalCode::EventLog,
    ] {
        assert_eq!(
            refusal(
                CallName::Input,
                &json!({}),
                &refusal_value(code, "Ignored."),
                AnswerAction::Answer
            )
            .message,
            None
        );
    }
    let r = refusal_value(RefusalCode::UnknownCall, "Unknown.");
    assert_eq!(
        refusal(CallName::SetModel, &json!({}), &r, AnswerAction::Answer)
            .message
            .as_deref(),
        Some("This server has no call setModel; it may be older than this client.")
    );
    let mut r = refusal_value(RefusalCode::BadRequest, "Invalid.");
    r.extra.insert(
        "errors".into(),
        json!([{"path":"/a","message":"missing"},{"path":"/b","message":"wrong"}]),
    );
    let c = refusal(CallName::Answer, &json!({}), &r, AnswerAction::Answer);
    assert_eq!(
        c.message.as_deref(),
        Some("Not recorded. The server refused it: Invalid.")
    );
    assert_eq!(c.errors.len(), 2);
    assert_eq!(c.errors[1].path, "/b");
    let r = refusal_value(
        RefusalCode::ExtensionNotLoaded,
        "Server wording is not parsed.",
    );
    assert_eq!(
        refusal(
            CallName::SetOptions,
            &json!({"extension":"custom"}),
            &r,
            AnswerAction::Answer
        )
        .message
        .as_deref(),
        Some("The server has no code for custom now.")
    );
    assert_eq!(
        refusal(CallName::Answer, &json!({}), &r, AnswerAction::Answer)
            .message
            .as_deref(),
        Some("Not recorded. The server refused it: Server wording is not parsed.")
    );
}

#[test]
fn outbox_states_and_retry_rules() {
    let unknown = || WriteState::Unknown {
        reason: Unknown::Timeout,
        next: Next::AtConnection,
    };
    for (call, word, expected, retry) in [
        (WriteCall::Input, "input", "no reply", true),
        (WriteCall::Continue, "continue", "no reply", true),
        (WriteCall::SetModel, "set model", "no reply", true),
        (WriteCall::SetOptions, "set options", "no reply", true),
        (WriteCall::Bind, "bind", "no reply", true),
        (WriteCall::AddExtensions, "add extensions", "no reply", true),
        (
            WriteCall::Answer,
            "answer",
            "no reply · open the decision to send it again",
            false,
        ),
        (
            WriteCall::Create,
            "create",
            "No reply · a conversation may have been created · try again from the start page",
            false,
        ),
        (
            WriteCall::Stop,
            "stop",
            "No reply · the run may have stopped",
            false,
        ),
    ] {
        let e = entry(call, unknown());
        let row = outbox_row(&e, Wall(0));
        assert_eq!(row.call, word);
        assert_eq!(row.state, expected);
        assert_eq!(row.retry.is_some(), retry);
        assert!(row.unknown);
        let mut restored = e.clone();
        restored.restored = true;
        let row = outbox_row(&restored, Wall(0));
        if matches!(call, WriteCall::SetModel | WriteCall::SetOptions) {
            assert_eq!(row.state, "may not have been applied · set it again");
            assert_eq!(row.retry, None);
        } else if retry {
            assert_eq!(row.state, "not sent automatically");
            assert_eq!(
                row.retry,
                Some(KeyHint {
                    key: "<leader>rt",
                    words: "retries"
                })
            );
        }
        if retry {
            let mut manual = e.clone();
            manual.state = WriteState::Unknown {
                reason: Unknown::Restart,
                next: Next::Manual,
            };
            assert_eq!(outbox_row(&manual, Wall(0)).state, "not sent automatically");
            assert_eq!(
                outbox_row(&e, Wall(600_001)).state,
                "not sent automatically"
            );
        }
    }
    for (state, words) in [
        (WriteState::Sending, "sending…"),
        (
            WriteState::Waiting {
                next: Next::AtConnection,
            },
            "waiting for the server",
        ),
        (
            WriteState::Done {
                at: Wall(0),
                result: json!({}),
            },
            "done",
        ),
        (WriteState::Settled(Settlement::NotLanded), "not recorded"),
        (
            WriteState::Refused(refusal_value(RefusalCode::TooLarge, "Too large.")),
            "Over 1 MiB · the server refused it · nothing was done",
        ),
    ] {
        assert_eq!(
            outbox_row(&entry(WriteCall::Input, state), Wall(0)).state,
            words
        );
    }
    let mut e = entry(WriteCall::Input, unknown());
    e.from_start = true;
    assert_eq!(
        outbox_row(&e, Wall(0)).state,
        "no reply · resends when connected"
    );
    assert_eq!(
        start_unknown(&json!({"project":"scratch"})),
        "? No reply · a conversation may have been created; it would show as empty conversation in scratch"
    );
    let d = record();
    assert_eq!(
        outbox_row(
            &entry(
                WriteCall::Answer,
                WriteState::Settled(Settlement::Landed(d.clone()))
            ),
            Wall(0)
        )
        .state,
        "recorded"
    );
    assert!(
        outbox_row(
            &entry(
                WriteCall::Answer,
                WriteState::Settled(Settlement::AnsweredElsewhere(d.clone()))
            ),
            Wall(0)
        )
        .state
        .starts_with("Not recorded. phone answered approve at ")
    );
    assert_eq!(
        outbox_row(
            &entry(
                WriteCall::Answer,
                WriteState::Settled(Settlement::NoLongerOpen(d))
            ),
            Wall(0)
        )
        .state,
        "Not recorded. This request is no longer open."
    );
}

#[test]
fn every_link_event_has_an_explicit_echo_policy() {
    let conv = ConvRef {
        project: Project::from("render-core"),
        id: ConversationId(3),
    };
    let decision = DecisionRef {
        project: conv.project.clone(),
        id: DecisionId::from("w1:3:design#2"),
    };
    let echo = |e| {
        event_echo(
            &e,
            &json!({"project":"render-core"}),
            Some("laptop"),
            AnswerAction::Answer,
        )
    };
    assert_eq!(
        echo(LinkEvent::DecisionLeft {
            decision: decision.clone(),
            ours: false
        })
        .as_deref(),
        Some("answered elsewhere")
    );
    for event in [
        LinkEvent::DecisionLeft {
            decision: decision.clone(),
            ours: true,
        },
        LinkEvent::Superseded {
            decision,
            by: DecisionId::from("w1:3:design#3"),
        },
        LinkEvent::Started {
            conv: conv.clone(),
            with: StartWith::Workflow("w1".into()),
            create: WriteId(1),
            input: WriteId(2),
        },
        LinkEvent::Revoked,
        LinkEvent::Paired {
            device: Device::from("laptop"),
        },
        LinkEvent::StreamEnded {
            key: StreamKey::Inbox,
            reason: EndReason::Revoked,
        },
        LinkEvent::StreamEnded {
            key: StreamKey::Inbox,
            reason: EndReason::Other("new".into()),
        },
    ] {
        assert_eq!(echo(event), None);
    }
    for key in [
        StreamKey::Inbox,
        StreamKey::Open(conv),
        StreamKey::Timeline(Project::from("render-core")),
    ] {
        let expected = if key == StreamKey::Inbox {
            "The list of open decisions failed on the server and is reopening."
        } else {
            "A stream failed on the server and is reopening."
        };
        assert_eq!(
            echo(LinkEvent::StreamEnded {
                key: key.clone(),
                reason: EndReason::Failed
            })
            .as_deref(),
            Some(expected)
        );
        assert_eq!(
            echo(LinkEvent::StreamEnded {
                key,
                reason: EndReason::Stopping
            })
            .as_deref(),
            Some("The server is stopping. tau reconnects when it is back.")
        );
    }
    let mut d = record();
    d.choice = Some("revise".into());
    assert_eq!(
        echo(LinkEvent::Write {
            id: WriteId(1),
            call: WriteCall::Answer,
            state: WriteState::Settled(Settlement::Landed(d.clone()))
        })
        .as_deref(),
        Some("Answered revise · the reply was lost, but it was recorded")
    );
    assert_eq!(
        echo(LinkEvent::Write {
            id: WriteId(1),
            call: WriteCall::Answer,
            state: WriteState::Done {
                at: Wall(0),
                result: json!({"decision":d,"repeated":false})
            }
        })
        .as_deref(),
        Some("Answered revise · w1.design_answer runs in #3")
    );
    let event = LinkEvent::Write {
        id: WriteId(1),
        call: WriteCall::Answer,
        state: WriteState::Done {
            at: Wall(0),
            result: json!({"decision":d,"repeated":true}),
        },
    };
    assert_eq!(
        echo(event.clone()).as_deref(),
        Some("Already recorded · by phone")
    );
    assert_eq!(
        event_echo(&event, &json!({}), Some("phone"), AnswerAction::Answer).as_deref(),
        Some("Already recorded")
    );
    assert_eq!(
        event_echo(
            &event,
            &json!({}),
            Some("phone"),
            AnswerAction::SendAgainAfterFault
        )
        .as_deref(),
        Some("Sent again · w1.design_answer runs again in #3")
    );
    for state in [
        WriteState::Sending,
        WriteState::Waiting { next: Next::Manual },
        WriteState::Done {
            at: Wall(0),
            result: json!({}),
        },
        WriteState::Unknown {
            reason: Unknown::Timeout,
            next: Next::AtConnection,
        },
        WriteState::Settled(Settlement::NotLanded),
    ] {
        assert_eq!(
            echo(LinkEvent::Write {
                id: WriteId(1),
                call: WriteCall::Input,
                state
            }),
            None
        );
    }
    assert_eq!(
        echo(LinkEvent::Write {
            id: WriteId(1),
            call: WriteCall::Continue,
            state: WriteState::Refused(refusal_value(RefusalCode::Busy, "Busy."))
        })
        .as_deref(),
        Some("A run is working; there is nothing to continue.")
    );
    assert_eq!(
        echo(LinkEvent::Write {
            id: WriteId(1),
            call: WriteCall::SetOptions,
            state: WriteState::Refused(refusal_value(
                RefusalCode::BadOptions,
                "The options of w1 do not fit."
            )),
        })
        .as_deref(),
        Some("The options of w1 do not fit.")
    );
    assert_eq!(
        echo(LinkEvent::PairFailed(PairFailure::NoReply(
            Unknown::OtherProtocol("2".into())
        ))),
        None,
        "Connect shows a pairing failure on its page"
    );
    assert_eq!(
        pair_failure(
            &PairFailure::NoReply(Unknown::OtherProtocol("2".into())),
            "https://tau.test",
            None
        ),
        "This server speaks protocol 2; this client speaks 1."
    );
}

#[test]
fn connect_distinguishes_proof_from_a_lost_reply() {
    let host = "This Host or Origin is not allowed.";
    let advice = "If you reach the server through a proxy, start it with tau --allow-host <name>";
    assert_eq!(
        pair_failure(
            &PairFailure::Refused(refusal_value(RefusalCode::Unauthorized, host)),
            "https://tau.test",
            Some("laptop")
        ),
        format!("{host} {advice}")
    );
    assert_eq!(
        refusal(
            CallName::Input,
            &json!({}),
            &refusal_value(RefusalCode::Unauthorized, host),
            AnswerAction::Answer
        )
        .message,
        Some(format!("The server refused this device: {host} {advice}"))
    );
    let wrong = "The code is wrong or expired.";
    assert_eq!(
        pair_failure(
            &PairFailure::Refused(refusal_value(RefusalCode::Unauthorized, wrong)),
            "https://tau.test",
            Some("laptop")
        ),
        format!(
            "{wrong} A code works once, for 10 minutes, and five wrong tries cancel every pending code. Run tau device pair <name> again."
        )
    );
    assert!(
        !refusal(
            CallName::Input,
            &json!({}),
            &refusal_value(RefusalCode::Unauthorized, "Host refused."),
            AnswerAction::Answer
        )
        .message
        .unwrap()
        .contains("allow-host")
    );
    let expected = "No reply from the server. If the code was accepted, the name laptop is now used and this device never got its token. Run tau device pair <new name> on the server, pair with the new code, then revoke the old name: tau device revoke laptop.";
    for reason in [
        Unknown::Timeout,
        Unknown::Restart,
        Unknown::Transport("cut".into()),
        Unknown::Unreadable { status: 502 },
        Unknown::Server {
            code: "storage_failed".into(),
            message: "Disk full".into(),
        },
    ] {
        assert_eq!(
            pair_failure(
                &PairFailure::NoReply(reason),
                "https://tau.test",
                Some("laptop")
            ),
            expected
        );
    }
    assert_eq!(
        pair_failure(
            &PairFailure::NoReply(Unknown::Timeout),
            "https://tau.test",
            None
        ),
        "No reply from the server. If the code was accepted, the name chosen on the server is now used and this device never got its token. Run tau device pair <new name> on the server, pair with the new code, then revoke the old name: tau device revoke <old name>."
    );
    assert_eq!(
        pair_failure(
            &PairFailure::NoReply(Unknown::Unreachable(
                "Connection refused (os error 61)".into()
            )),
            "http://127.0.0.1:6283",
            Some("laptop")
        ),
        "No tau server answered at http://127.0.0.1:6283: Connection refused (os error 61). Check the address, and that tau is running there."
    );
    assert_eq!(
        no_server("https://tau.test", "connection refused."),
        "No tau server answered at https://tau.test: connection refused. Check the address, and that tau is running there."
    );
    assert_eq!(
        no_server("https://tau.test", "connection refused"),
        "No tau server answered at https://tau.test: connection refused. Check the address, and that tau is running there."
    );
}
