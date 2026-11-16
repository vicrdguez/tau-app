//! Calls and streams over HTTP/1.1, plain or TLS. A small client of its own, because a stream must be closable
//! from another thread at once: the socket it reads is kept so a close can shut it down.

use crate::engine::StreamEvent;
use crate::sse::SseParser;
use crate::transport::{Deliver, Reply, StreamHandle, Target, Transport};
use crate::wire::{Outcome, Request, Token, Unknown, parse_reply};
use rustls::pki_types::ServerName;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// A call's reply must come within this; the engine applies the same rule to the outcome.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(20);
/// The longest reply body read for a call.
const REPLY_LIMIT: u64 = 64 * 1024 * 1024;

/// A server URL: `http://` or `https://`, a host, an optional port and an optional path prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerUrl {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    /// A path prefix without its trailing slash, such as a proxy's mount point; usually empty.
    pub base: String,
}

impl ServerUrl {
    pub fn parse(text: &str) -> Result<ServerUrl, String> {
        let bad = || format!("{text} is not an http:// or https:// address");
        let (tls, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(bad());
        };
        let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
        let (host, port) = if authority.starts_with('[') {
            let close = authority.find(']').ok_or_else(bad)?;
            let port = authority[close + 1..].strip_prefix(':');
            (&authority[..=close], port)
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };
        if host.is_empty() {
            return Err(bad());
        }
        let port = match port {
            Some(p) => p.parse().map_err(|_| bad())?,
            None if tls => 443,
            None => 80,
        };
        let base = path.trim_end_matches('/');
        let base = if base.is_empty() {
            String::new()
        } else {
            format!("/{base}")
        };
        Ok(ServerUrl {
            tls,
            host: host.to_ascii_lowercase(),
            port,
            base,
        })
    }

    /// Plain HTTP to a host that is not `127.0.0.1`, `localhost` or `[::1]`: anyone on the path can read the token
    /// (`client.md` §7).
    pub fn is_exposed(&self) -> bool {
        !self.tls && !["127.0.0.1", "localhost", "[::1]"].contains(&self.host.as_str())
    }

    fn host_header(&self) -> String {
        let default = if self.tls { 443 } else { 80 };
        if self.port == default {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

/// The transport of a real server: a thread per call, a thread per stream.
pub struct HttpTransport;

impl Transport for HttpTransport {
    fn call(&self, target: &Target, request: Request, reply: Reply) {
        let target = target.clone();
        thread::spawn(move || reply(call(&target.url, target.token.as_ref(), &request)));
    }

    fn open(&self, target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle> {
        let shared = Arc::new(Shared {
            socket: Mutex::new(None),
            closed: AtomicBool::new(false),
        });
        let target = target.clone();
        let mine = shared.clone();
        thread::spawn(move || stream(&target, &request, &deliver, &mine));
        Box::new(HttpStream(shared))
    }
}

/// One call, blocking: the outcome its reply proves.
pub fn call(url: &str, token: Option<&Token>, request: &Request) -> Outcome {
    let unknown = |e: io::Error| Outcome::Unknown(Unknown::Transport(e.to_string()));
    let unreachable = |e: String| Outcome::Unknown(Unknown::Unreachable(e));
    let url = match ServerUrl::parse(url) {
        Ok(url) => url,
        Err(e) => return unreachable(e),
    };
    let tcp = match connect(&url, Some(CALL_TIMEOUT)) {
        Ok(tcp) => tcp,
        Err(e) => return unreachable(e.to_string()),
    };
    let reply =
        post(tcp, &url, request, token, Some(CALL_TIMEOUT)).and_then(|(mut response, _)| {
            let mut body = Vec::new();
            response
                .body
                .by_ref()
                .take(REPLY_LIMIT)
                .read_to_end(&mut body)?;
            Ok((response, body))
        });
    match reply {
        Ok((response, body)) => {
            parse_reply(response.status, response.header("tau-protocol"), &body)
        }
        Err(e) => unknown(e),
    }
}

struct Shared {
    socket: Mutex<Option<TcpStream>>,
    closed: AtomicBool,
}

struct HttpStream(Arc<Shared>);

impl StreamHandle for HttpStream {
    fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
        if let Some(socket) = self
            .0
            .socket
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

/// Runs one stream until it ends or is closed. Nothing is delivered after a close.
fn stream(target: &Target, request: &Request, deliver: &Deliver, shared: &Shared) {
    let send = |event: StreamEvent| {
        if !shared.closed.load(Ordering::SeqCst) {
            deliver(event);
        }
    };
    let url = match ServerUrl::parse(&target.url) {
        Ok(url) => url,
        Err(e) => return send(StreamEvent::Closed(Some(e))),
    };
    let opened =
        connect(&url, None).and_then(|tcp| post(tcp, &url, request, target.token.as_ref(), None));
    let (mut response, socket) = match opened {
        Ok(opened) => opened,
        Err(e) => return send(StreamEvent::Closed(Some(e.to_string()))),
    };
    {
        let mut slot = shared.socket.lock().unwrap_or_else(|e| e.into_inner());
        if shared.closed.load(Ordering::SeqCst) {
            let _ = socket.shutdown(Shutdown::Both);
            return;
        }
        *slot = Some(socket);
    }
    let events = response
        .header("content-type")
        .is_some_and(|t| t.starts_with("text/event-stream"));
    let protocol = response.header("tau-protocol").map(str::to_owned);
    if !events
        || protocol
            .as_deref()
            .is_some_and(|p| p.trim() != crate::wire::PROTOCOL)
    {
        let mut body = Vec::new();
        let _ = response
            .body
            .by_ref()
            .take(REPLY_LIMIT)
            .read_to_end(&mut body);
        return send(StreamEvent::Refused(parse_reply(
            response.status,
            protocol.as_deref(),
            &body,
        )));
    }
    let mut parser = SseParser::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        match response.body.read(&mut buf) {
            Ok(0) => return send(StreamEvent::Closed(None)),
            Ok(n) => {
                for frame in parser.feed(&buf[..n]) {
                    send(match frame {
                        Ok(frame) => StreamEvent::Frame(frame),
                        Err(bad) => StreamEvent::BadFrame(bad.0),
                    });
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return send(StreamEvent::Closed(Some(e.to_string()))),
        }
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Box<dyn Read + Send>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

trait Conn: Read + Write + Send {}
impl<T: Read + Write + Send> Conn for T {}

/// Sends one `POST /v1/<call>` and reads the reply's head. Returns a clone of the socket so another thread can shut
/// the connection down. `timeout` bounds connecting and every read and write; `None` reads until the server or a
/// close ends the stream.
/// A TCP connection to the server's host: nothing has been sent when this fails.
fn connect(url: &ServerUrl, timeout: Option<Duration>) -> io::Result<TcpStream> {
    let host = url.host.trim_start_matches('[').trim_end_matches(']');
    let addrs: Vec<_> = (host, url.port).to_socket_addrs()?.collect();
    let mut last = io::Error::new(io::ErrorKind::NotFound, format!("{host} has no address"));
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout.unwrap_or(CALL_TIMEOUT)) {
            Ok(s) => return Ok(s),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn post(
    tcp: TcpStream,
    url: &ServerUrl,
    request: &Request,
    token: Option<&Token>,
    timeout: Option<Duration>,
) -> io::Result<(Response, TcpStream)> {
    let host = url.host.trim_start_matches('[').trim_end_matches(']');
    tcp.set_nodelay(true)?;
    tcp.set_read_timeout(timeout)?;
    tcp.set_write_timeout(Some(timeout.unwrap_or(CALL_TIMEOUT)))?;
    let socket = tcp.try_clone()?;
    let mut conn: Box<dyn Conn> = if url.tls {
        let name = ServerName::try_from(host.to_owned())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let tls = rustls::ClientConnection::new(tls_config()?, name).map_err(io::Error::other)?;
        Box::new(rustls::StreamOwned::new(tls, tcp))
    } else {
        Box::new(tcp)
    };
    let body = serde_json::to_vec(&request.body)?;
    let mut head = format!(
        "POST {}/v1/{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: tau-desktop\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        url.base,
        request.call.path(),
        url.host_header(),
        body.len()
    );
    if let Some(token) = token {
        head.push_str(&format!("Authorization: Bearer {}\r\n", token.expose()));
    }
    head.push_str("\r\n");
    conn.write_all(head.as_bytes())?;
    conn.write_all(&body)?;
    conn.flush()?;
    let mut reader = BufReader::new(conn);
    let status = read_status(&mut reader)?;
    let headers = read_headers(&mut reader)?;
    let find = |name: &str| {
        headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let body: Box<dyn Read + Send> = if find("transfer-encoding")
        .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"))
    {
        Box::new(Chunked {
            inner: reader,
            left: 0,
            state: ChunkState::Size,
        })
    } else if let Some(len) = find("content-length").and_then(|v| v.trim().parse::<u64>().ok()) {
        Box::new(reader.take(len))
    } else {
        Box::new(reader)
    };
    Ok((
        Response {
            status,
            headers,
            body,
        },
        socket,
    ))
}

fn read_line(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = String::new();
    if reader.by_ref().take(64 * 1024).read_line(&mut line)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the connection closed",
        ));
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

fn read_status(reader: &mut impl BufRead) -> io::Result<u16> {
    let line = read_line(reader)?;
    let mut parts = line.split(' ');
    let code = match (parts.next(), parts.next()) {
        (Some(version), Some(code)) if version.starts_with("HTTP/") => code.parse().ok(),
        _ => None,
    };
    code.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("not an HTTP reply: {line}"),
        )
    })
}

fn read_headers(reader: &mut impl BufRead) -> io::Result<Vec<(String, String)>> {
    let mut headers = Vec::new();
    loop {
        let line = read_line(reader)?;
        if line.is_empty() {
            return Ok(headers);
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ChunkState {
    Size,
    Data,
    Done,
}

/// `Transfer-Encoding: chunked`, decoded.
struct Chunked<R> {
    inner: R,
    left: u64,
    state: ChunkState,
}

impl<R: BufRead> Read for Chunked<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.state {
                ChunkState::Done => return Ok(0),
                ChunkState::Size => {
                    let line = read_line(&mut self.inner)?;
                    let size = line.split(';').next().unwrap_or_default().trim();
                    self.left = u64::from_str_radix(size, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("bad chunk size {line}"))
                    })?;
                    if self.left == 0 {
                        while !read_line(&mut self.inner)?.is_empty() {}
                        self.state = ChunkState::Done;
                    } else {
                        self.state = ChunkState::Data;
                    }
                }
                ChunkState::Data => {
                    let want = buf.len().min(self.left as usize);
                    let n = self.inner.read(&mut buf[..want])?;
                    if n == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "the connection closed inside a chunk",
                        ));
                    }
                    self.left -= n as u64;
                    if self.left == 0 {
                        read_line(&mut self.inner)?;
                        self.state = ChunkState::Size;
                    }
                    return Ok(n);
                }
            }
        }
    }
}

/// TLS that trusts what the system trusts: on macOS the Keychain's roots and the user's own, so a Tailscale
/// certificate or a proxy's private CA both work.
fn tls_config() -> io::Result<Arc<rustls::ClientConfig>> {
    static CONFIG: OnceLock<Result<Arc<rustls::ClientConfig>, String>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            use rustls_platform_verifier::BuilderVerifierExt;
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .and_then(|b| b.with_platform_verifier())
                .map(|b| Arc::new(b.with_no_client_auth()))
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(io::Error::other)
}
