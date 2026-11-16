use serde_json::json;
use tau_client::sse::{EndReason, Frame, SseParser};

fn frames(chunks: &[&[u8]]) -> Vec<Frame> {
    let mut p = SseParser::new();
    chunks
        .iter()
        .flat_map(|c| p.feed(c))
        .map(|f| f.expect("a good frame"))
        .collect()
}

fn recorded(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/e2e/captures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let body = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a response head")
        + 4;
    raw[body..].to_vec()
}

#[test]
fn frames_survive_any_split() {
    let text = "data: {\"ops\":[[\"a\",[\"t\"],\"✔ ok\"]]}\n\n:\n\ndata: {\"end\":{\"reason\":\"failed\"}}\n\n".as_bytes();
    let whole = frames(&[text]);
    assert_eq!(whole.len(), 3);
    for at in 0..text.len() {
        assert_eq!(frames(&[&text[..at], &text[at..]]), whole, "split at {at}");
    }
    let bytes: Vec<&[u8]> = text.chunks(1).collect();
    assert_eq!(frames(&bytes), whole);
}

#[test]
fn heartbeat_is_a_frame() {
    assert_eq!(frames(&[b":\n\n"]), vec![Frame::Heartbeat]);
}

#[test]
fn line_ends_and_spacing_vary() {
    let want = vec![Frame::Ops(vec![json!(["r", 1])])];
    assert_eq!(frames(&[b"data:{\"ops\":[[\"r\",1]]}\n\n"]), want);
    assert_eq!(frames(&[b"data: {\"ops\":[[\"r\",1]]}\r\n\r\n"]), want);
    assert_eq!(frames(&[b"data: {\"ops\":[[\"r\",1]]}\r\r"]), want);
    assert_eq!(frames(&[b"data: {\"ops\":[[\"r\",1]]}\r", b"\n\r\n"]), want);
}

#[test]
fn data_lines_join() {
    assert_eq!(
        frames(&[b"data: {\"ops\":\ndata: [[\"r\",1]]}\n\n"]),
        vec![Frame::Ops(vec![json!(["r", 1])])]
    );
}

#[test]
fn end_reasons() {
    let got = frames(&[
        b"data: {\"end\":{\"reason\":\"stopping\"}}\n\n",
        b"data: {\"end\":{\"reason\":\"revoked\"}}\n\n",
        b"data: {\"end\":{\"reason\":\"failed\"}}\n\n",
        b"data: {\"end\":{\"reason\":\"melted\"}}\n\n",
    ]);
    assert_eq!(
        got,
        vec![
            Frame::End(EndReason::Stopping),
            Frame::End(EndReason::Revoked),
            Frame::End(EndReason::Failed),
            Frame::End(EndReason::Other("melted".into())),
        ]
    );
}

#[test]
fn a_bad_frame_does_not_stop_the_stream() {
    let mut p = SseParser::new();
    let got = p.feed(b"data: {nope\n\ndata: {\"x\":1}\n\ndata: {\"ops\":[[\"r\",2]]}\n\n");
    assert_eq!(got.len(), 3);
    assert!(got[0].is_err() && got[1].is_err());
    assert_eq!(got[2], Ok(Frame::Ops(vec![json!(["r", 2])])));
}

#[test]
fn an_unfinished_frame_waits() {
    let mut p = SseParser::new();
    assert!(p.feed(b"data: {\"ops\":[[\"r\",1]]}\n").is_empty());
    assert_eq!(p.feed(b"\n").len(), 1);
}

#[test]
fn recorded_streams_parse() {
    let open = frames(&[&recorded("stream-faux-open.raw")]);
    assert_eq!(open.len(), 6);
    assert!(matches!(&open[0], Frame::Ops(ops) if ops[0][0] == "r"));
    let inbox = frames(&[&recorded("stream-inbox.raw")]);
    assert_eq!(inbox.last(), Some(&Frame::Heartbeat));
    let revoked = frames(&[&recorded("stream-inbox-revoked.raw")]);
    assert_eq!(revoked.last(), Some(&Frame::End(EndReason::Revoked)));
    let stopping = frames(&[&recorded("stream-open-after-stopping.raw")]);
    assert_eq!(stopping.last(), Some(&Frame::End(EndReason::Stopping)));
}
