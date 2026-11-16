use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tau_client::engine::StreamEvent;
use tau_client::http::{HttpTransport, ServerUrl, call};
use tau_client::sse::Frame;
use tau_client::transport::{Target, Transport};
use tau_client::wire::{CallName, Outcome, Request, Token, Unknown};

/// One request as a server reads it: the head's lines and the body.
struct Seen {
    head: Vec<String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Seen {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut head = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        head.push(line);
    }
    let len: usize = head
        .iter()
        .find_map(|h| {
            h.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; len];
    reader.read_exact(&mut body).unwrap();
    Seen {
        head,
        body: String::from_utf8(body).unwrap(),
    }
}

/// A server that answers one request with `reply` and reports what it read.
fn serve_once(reply: &'static [u8]) -> (String, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let seen = read_request(&mut stream);
        stream.write_all(reply).unwrap();
        tx.send(seen).unwrap();
    });
    (url, rx)
}

fn projects() -> Request {
    Request {
        call: CallName::Projects,
        body: json!({}),
    }
}

#[test]
fn http_calls_carry_their_headers() {
    let (url, seen) = serve_once(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTau-Protocol: 1\r\nContent-Length: 13\r\n\r\n{\"result\":{}}");
    let token = Token::new("tau.laptop.Qm9vVGF1RGVza3RvcFRva2VuRm9yVGhlRml4dHVyZTA");
    assert_eq!(
        call(&url, Some(&token), &projects()),
        Outcome::Done(json!({}))
    );
    let seen = seen.recv().unwrap();
    assert_eq!(seen.head[0], "POST /v1/projects HTTP/1.1");
    let has = |h: &str| seen.head.iter().any(|l| l.eq_ignore_ascii_case(h));
    assert!(has(&format!("Authorization: Bearer {}", token.expose())));
    assert!(has("Content-Type: application/json"));
    assert!(
        !seen
            .head
            .iter()
            .any(|l| l.to_ascii_lowercase().starts_with("origin:")),
        "the client sends no Origin"
    );
    assert_eq!(seen.body, "{}");
}

#[test]
fn pair_sends_no_token_and_the_handshake_posts_to_v1() {
    let (url, seen) = serve_once(b"HTTP/1.1 404 Not Found\r\nTau-Protocol: 1\r\nContent-Length: 66\r\n\r\n{\"refused\":{\"code\":\"unknown_call\",\"message\":\"x\",\"calls\":[\"open\"]}}");
    let outcome = call(
        &url,
        None,
        &Request {
            call: CallName::Handshake,
            body: json!({}),
        },
    );
    assert!(matches!(outcome, Outcome::Refused(r) if r.calls() == ["open"]));
    let seen = seen.recv().unwrap();
    assert_eq!(seen.head[0], "POST /v1/ HTTP/1.1");
    assert!(
        !seen
            .head
            .iter()
            .any(|l| l.to_ascii_lowercase().starts_with("authorization:"))
    );
}

#[test]
fn chunked_replies_and_other_protocols() {
    let (url, _) = serve_once(b"HTTP/1.1 200 OK\r\nTau-Protocol: 1\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n{\"res\r\n8\r\nult\":{}}\r\n0\r\n\r\n");
    assert_eq!(call(&url, None, &projects()), Outcome::Done(json!({})));
    let (url, _) = serve_once(
        b"HTTP/1.1 200 OK\r\nTau-Protocol: 2\r\nContent-Length: 13\r\n\r\n{\"result\":{}}",
    );
    assert_eq!(
        call(&url, None, &projects()),
        Outcome::Unknown(Unknown::OtherProtocol("2".into()))
    );
    let (url, _) = serve_once(b"HTTP/1.1 409 Conflict\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html>proxy</html>");
    assert_eq!(
        call(&url, None, &projects()),
        Outcome::Unknown(Unknown::Unreadable { status: 409 })
    );
}

#[test]
fn no_server_is_unreachable_so_nothing_was_sent() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let outcome = call(&format!("http://127.0.0.1:{port}"), None, &projects());
    assert!(
        matches!(outcome, Outcome::Unknown(Unknown::Unreachable(_))),
        "{outcome:?}"
    );
    let bad = call("tau.example", None, &projects());
    assert!(
        matches!(bad, Outcome::Unknown(Unknown::Unreachable(_))),
        "{bad:?}"
    );
}

#[test]
fn urls() {
    let u = ServerUrl::parse("https://tau.tail4e2f.ts.net").unwrap();
    assert_eq!(
        (u.tls, u.host.as_str(), u.port, u.base.as_str()),
        (true, "tau.tail4e2f.ts.net", 443, "")
    );
    let u = ServerUrl::parse("http://127.0.0.1:6283/").unwrap();
    assert_eq!((u.tls, u.port, u.base.as_str()), (false, 6283, ""));
    let u = ServerUrl::parse("http://[::1]:6283/tau").unwrap();
    assert_eq!((u.host.as_str(), u.base.as_str()), ("[::1]", "/tau"));
    assert!(!u.is_exposed());
    assert!(
        !ServerUrl::parse("http://localhost:6283")
            .unwrap()
            .is_exposed()
    );
    assert!(
        ServerUrl::parse("http://192.168.1.4:6283")
            .unwrap()
            .is_exposed()
    );
    assert!(
        !ServerUrl::parse("https://192.168.1.4")
            .unwrap()
            .is_exposed()
    );
    for bad in ["ftp://x", "tau.local", "http://", "https://:80"] {
        assert!(ServerUrl::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn streams_close_within_a_second() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (gone_tx, gone) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_request(&mut stream);
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTau-Protocol: 1\r\nTransfer-Encoding: chunked\r\n\r\n";
        let frame = "data: {\"ops\":[[\"r\",{}]]}\n\n";
        stream.write_all(head.as_bytes()).unwrap();
        stream
            .write_all(format!("{:x}\r\n{frame}\r\n", frame.len()).as_bytes())
            .unwrap();
        let mut rest = Vec::new();
        let _ = stream.read_to_end(&mut rest);
        gone_tx.send(Instant::now()).unwrap();
    });
    let (tx, events) = mpsc::channel();
    let target = Target { url, token: None };
    let request = Request {
        call: CallName::Inbox,
        body: json!({}),
    };
    let handle = HttpTransport.open(&target, request, Box::new(move |e| tx.send(e).unwrap()));
    let first = events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        first,
        StreamEvent::Frame(Frame::Ops(vec![json!(["r", {}])]))
    );
    let closed = Instant::now();
    handle.close();
    let at = gone
        .recv_timeout(Duration::from_secs(2))
        .expect("the server sees the connection close");
    assert!(at.duration_since(closed) < Duration::from_secs(1));
    assert!(
        events.recv_timeout(Duration::from_millis(200)).is_err(),
        "nothing is delivered after a close"
    );
}

#[test]
fn a_stream_cut_by_the_server_is_reported() {
    let (url, _) = serve_once(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTau-Protocol: 1\r\n\r\ndata: {\"ops\":[[\"r\",1]]}\n\n:\n\n");
    let (tx, events) = mpsc::channel();
    let request = Request {
        call: CallName::Inbox,
        body: json!({}),
    };
    let _handle = HttpTransport.open(
        &Target { url, token: None },
        request,
        Box::new(move |e| tx.send(e).unwrap()),
    );
    let got: Vec<StreamEvent> = (0..3)
        .map(|_| events.recv_timeout(Duration::from_secs(5)).unwrap())
        .collect();
    assert_eq!(
        got[0],
        StreamEvent::Frame(Frame::Ops(vec![json!(["r", 1])]))
    );
    assert_eq!(got[1], StreamEvent::Frame(Frame::Heartbeat));
    assert!(matches!(got[2], StreamEvent::Closed(_)));
}

#[test]
fn a_stream_refused_before_it_starts() {
    let (url, _) = serve_once(b"HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nTau-Protocol: 1\r\nContent-Length: 54\r\n\r\n{\"refused\":{\"code\":\"busy\",\"message\":\"The server is.\"}}");
    let (tx, events) = mpsc::channel();
    let request = Request {
        call: CallName::Inbox,
        body: json!({}),
    };
    let _handle = HttpTransport.open(
        &Target { url, token: None },
        request,
        Box::new(move |e| tx.send(e).unwrap()),
    );
    let got = events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        matches!(&got, StreamEvent::Refused(Outcome::Refused(r)) if r.code.as_str() == "busy"),
        "{got:?}"
    );
}
