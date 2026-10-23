//! Unix sockets: a one-shot request, a connection that stays open, and a server.
//!
//! Operations:
//! - `sock.request` `{ path, timeout?, halfClose? }` connects, writes the request's bytes,
//!   closes its writing end (unless `halfClose` is false), reads to the end and settles
//!   with the bytes it read. It gives up after `timeout` milliseconds (30 s by default).
//! - `sock.connect` `{ path }` is a stream: `{ type: "open" }`, then `{ type: "line", line }`
//!   for each line the peer sends, and `close` when either side has ended it.
//!   `sock.write` `{ socket }` (bytes) queues data to send, and `sock.close` `{ socket }`
//!   closes the connection once what was queued is out.
//! - `sock.serve` `{ path, framing }` listens on a path, removing a socket file nobody
//!   answers on first. It is a stream: `{ type: "listening" }`, then `{ type: "request",
//!   id, peer, text }` for each request, with `{ type: "peer", peer }` before a client's first
//!   and `{ type: "gone", peer }` after its last. The app answers a request with `sock.reply`
//!   `{ request, failed? }` (bytes). It can write to a client at any time with `sock.push`
//!   `{ server, peer }` (bytes), and end that connection with `sock.hangup` `{ server, peer }`.
//!   `sock.stop` `{ server }` ends the server and removes its file.
//!   With framing `"eof"` a request is everything the peer sends before it closes its
//!   writing end, and the answer ends the connection. With `"line"` every line is a
//!   request, answered with a line, and the connection stays until the peer leaves.
//! - `sock.limit` `{ allow }` limits which paths may be used, once.

use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use polling::{Event, Events, Poller};
use serde_json::{Value, json};

use super::Sink;
use super::files::{Directories, canonical};
use super::lines::LineSplitter;

/// How long a request waits for the answer when the app names no limit.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long a server waits for a peer that has connected to say anything in `"eof"` framing.
const PEER_IDLE: Duration = Duration::from_secs(30);

/// The most a server reads from one peer in `"eof"` framing.
const MOST_REQUEST: usize = 16 << 20;

struct SocketError {
    code: &'static str,
    message: String,
}

impl SocketError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        SocketError { code, message: message.into() }
    }

    fn system(action: &str, path: &str, error: std::io::Error) -> Self {
        let code = match error.kind() {
            ErrorKind::NotFound => "NotFound",
            ErrorKind::PermissionDenied => "PermissionDenied",
            ErrorKind::ConnectionRefused => "ConnectionRefused",
            ErrorKind::AddrInUse => "AddressInUse",
            ErrorKind::TimedOut | ErrorKind::WouldBlock => "TimedOut",
            _ => "Other",
        };
        SocketError::new(code, format!("cannot {action} {path}: {error}"))
    }

    fn not_allowed(path: &str) -> Self {
        SocketError::new("NotAllowed", format!("access to {path} is not allowed"))
    }

    fn argument(message: impl Into<String>) -> Self {
        SocketError::new("InvalidArgument", message)
    }
}

enum Output {
    Data(Vec<u8>),
    Close,
}

/// A connection the app opened.
struct Connection {
    output: Sender<Output>,
    /// A handle on the socket, once connected, for ending it from outside its threads.
    stream: Mutex<Option<UnixStream>>,
}

struct Server {
    stop: AtomicBool,
    poller: Poller,
    /// The peers connected now, for ending them with the server and for writing to them.
    peers: Mutex<HashMap<u64, Arc<Peer>>>,
}

/// A client of a server of the app's.
struct Peer {
    stream: UnixStream,
    /// Held while writing, so an answer and a push never mix their bytes.
    writing: Arc<Mutex<()>>,
    /// What the app pushed, in order, written by a thread of its own so the app never waits on a slow peer.
    pushed: Sender<Output>,
}

#[derive(Clone, Copy, PartialEq)]
enum Framing {
    Eof,
    Line,
}

enum Answer {
    Data(Vec<u8>),
    Failed,
}

/// A request waiting for the app's answer.
struct Pending {
    server: u64,
    answer: Sender<Answer>,
}

#[derive(Clone)]
pub struct Unix {
    directories: Arc<Directories>,
    allowed: Arc<Mutex<Option<Vec<PathBuf>>>>,
    connections: Arc<Mutex<HashMap<u64, Arc<Connection>>>>,
    servers: Arc<Mutex<HashMap<u64, Arc<Server>>>>,
    pending: Arc<Mutex<HashMap<u64, Pending>>>,
    last_id: Arc<AtomicU64>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Unix {
    pub fn new(directories: Directories) -> Self {
        Unix {
            directories: Arc::new(directories),
            allowed: Arc::default(),
            connections: Arc::default(),
            servers: Arc::default(),
            pending: Arc::default(),
            last_id: Arc::default(),
        }
    }

    pub fn request(&self, sink: &Sink, op: &str, call: u64, args: Value, bytes: Option<Vec<u8>>) {
        match op {
            "sock.request" => self.spawn(sink, call, move |unix| unix.exchange(&args, bytes.unwrap_or_default())),
            "sock.connect" => self.connect(sink, call, args),
            "sock.write" => self.send(sink, call, &args, Output::Data(bytes.unwrap_or_default())),
            "sock.close" => self.send(sink, call, &args, Output::Close),
            "sock.serve" => self.serve(sink, call, args),
            "sock.reply" => self.reply(sink, call, &args, bytes),
            "sock.push" => self.push(sink, call, &args, Output::Data(bytes.unwrap_or_default())),
            "sock.hangup" => self.push(sink, call, &args, Output::Close),
            "sock.stop" => self.stop(sink, call, &args),
            "sock.limit" => match self.declare_limit(&args) {
                Ok(()) => sink.resolve(call, json!({})),
                Err(error) => sink.reject_with_code(call, error.code, error.message),
            },
            _ => sink.reject(call, format!("svelte-gpui: unknown operation {op}")),
        }
    }

    /// Ends every connection and server, for an app that is going away.
    pub fn close_all(&self) {
        for (_, connection) in locked(&self.connections).drain() {
            if let Some(stream) = locked(&connection.stream).as_ref() {
                let _ = stream.shutdown(Shutdown::Both);
            }
            let _ = connection.output.send(Output::Close);
        }
        for (_, server) in locked(&self.servers).drain() {
            server.stop.store(true, Ordering::Relaxed);
            let _ = server.poller.notify();
        }
        locked(&self.pending).clear();
    }

    /// Runs a call that settles once on a thread.
    fn spawn(&self, sink: &Sink, call: u64, work: impl FnOnce(&Unix) -> Result<Vec<u8>, SocketError> + Send + 'static) {
        let (unix, sink) = (self.clone(), sink.clone());
        std::thread::spawn(move || match work(&unix) {
            Ok(bytes) => sink.resolve_bytes(call, json!({}), bytes),
            Err(error) => sink.reject_with_code(call, error.code, error.message),
        });
    }

    fn declare_limit(&self, args: &Value) -> Result<(), SocketError> {
        let mut roots = Vec::new();
        for entry in args["allow"].as_array().into_iter().flatten() {
            let raw = entry.as_str().ok_or_else(|| SocketError::argument("allow holds only paths"))?;
            let path = self.directories.expand_allowed(raw).map_err(SocketError::argument)?;
            roots.push(canonical(&path, true));
        }
        let mut allowed = locked(&self.allowed);
        if allowed.is_some() {
            return Err(SocketError::new("NotAllowed", "the socket limit is already set and cannot be changed"));
        }
        *allowed = Some(roots);
        Ok(())
    }

    /// The path the app named, made absolute, if the limit allows it.
    fn path(&self, args: &Value) -> Result<(String, PathBuf), SocketError> {
        let raw = args["path"].as_str().ok_or_else(|| SocketError::argument("a socket needs a path"))?;
        let path = self.directories.expand(raw);
        if let Some(roots) = locked(&self.allowed).as_ref() {
            let target = canonical(&path, true);
            if !roots.iter().any(|root| target.starts_with(root)) {
                return Err(SocketError::not_allowed(raw));
            }
        }
        Ok((raw.to_string(), path))
    }

    fn exchange(&self, args: &Value, request: Vec<u8>) -> Result<Vec<u8>, SocketError> {
        let (raw, path) = self.path(args)?;
        let timeout = args["timeout"].as_u64().map(Duration::from_millis).unwrap_or(REQUEST_TIMEOUT);
        let deadline = Instant::now() + timeout;
        let mut stream = UnixStream::connect(&path).map_err(|error| SocketError::system("connect to", &raw, error))?;
        let timed_out =
            || SocketError::new("TimedOut", format!("{raw} did not answer within {} ms", timeout.as_millis()));
        stream.set_write_timeout(Some(timeout)).map_err(|error| SocketError::system("write to", &raw, error))?;
        stream.write_all(&request).map_err(|error| SocketError::system("write to", &raw, error))?;
        if args["halfClose"].as_bool().unwrap_or(true) {
            let _ = stream.shutdown(Shutdown::Write);
        }
        let mut answer = Vec::new();
        let mut buffer = [0u8; 16384];
        loop {
            let left = deadline.checked_duration_since(Instant::now()).filter(|left| !left.is_zero());
            stream
                .set_read_timeout(Some(left.ok_or_else(timed_out)?))
                .map_err(|error| SocketError::system("read from", &raw, error))?;
            match stream.read(&mut buffer) {
                Ok(0) => return Ok(answer),
                Ok(count) => answer.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return Err(timed_out());
                }
                Err(error) => return Err(SocketError::system("read from", &raw, error)),
            }
        }
    }

    fn connect(&self, sink: &Sink, call: u64, args: Value) {
        let (output, queued) = channel();
        let connection = Arc::new(Connection { output, stream: Mutex::new(None) });
        locked(&self.connections).insert(call, connection.clone());
        let (unix, sink) = (self.clone(), sink.clone());
        std::thread::spawn(move || {
            let opened = unix.path(&args).and_then(|(raw, path)| {
                UnixStream::connect(&path).map_err(|error| SocketError::system("connect to", &raw, error))
            });
            let stream = match opened {
                Ok(stream) => stream,
                Err(error) => {
                    locked(&unix.connections).remove(&call);
                    sink.reject_with_code(call, error.code, error.message);
                    return;
                }
            };
            if let (Ok(writer), Ok(shutdown)) = (stream.try_clone(), stream.try_clone()) {
                *locked(&connection.stream) = Some(shutdown);
                std::thread::spawn(move || write_queued(writer, &Mutex::new(()), queued));
            }
            sink.event(call, json!({ "type": "open" }), None);
            super::lines::for_each_line(&stream, |line| {
                sink.event(call, json!({ "type": "line", "line": line }), None)
            });
            locked(&unix.connections).remove(&call);
            drop(connection);
            sink.close(call, json!({}));
        });
    }

    fn send(&self, sink: &Sink, call: u64, args: &Value, output: Output) {
        let connection = args["socket"].as_u64().and_then(|id| locked(&self.connections).get(&id).cloned());
        if let Some(connection) = connection {
            let _ = connection.output.send(output);
        }
        sink.resolve(call, json!({}));
    }

    fn serve(&self, sink: &Sink, call: u64, args: Value) {
        let framing = if args["framing"].as_str() == Some("line") { Framing::Line } else { Framing::Eof };
        let poller = match Poller::new() {
            Ok(poller) => poller,
            Err(error) => return sink.reject(call, format!("could not start the server: {error}")),
        };
        let server = Arc::new(Server { stop: AtomicBool::new(false), poller, peers: Mutex::default() });
        locked(&self.servers).insert(call, server.clone());
        let (unix, sink) = (self.clone(), sink.clone());
        std::thread::spawn(move || {
            let bound = unix.path(&args).and_then(|(raw, path)| bind(&raw, &path).map(|listener| (listener, path)));
            match bound {
                Ok((listener, path)) => unix.accept_until_stopped(&sink, call, &server, listener, &path, framing),
                Err(error) => sink.reject_with_code(call, error.code, error.message),
            }
            server.stop.store(true, Ordering::Relaxed);
            for (_, peer) in locked(&server.peers).drain() {
                let _ = peer.stream.shutdown(Shutdown::Both);
            }
            locked(&unix.pending).retain(|_, pending| pending.server != call);
            locked(&unix.servers).remove(&call);
        });
    }

    fn accept_until_stopped(
        &self,
        sink: &Sink,
        call: u64,
        server: &Arc<Server>,
        listener: UnixListener,
        path: &Path,
        framing: Framing,
    ) {
        // Safety: the listener stays open until it is removed from the poller below.
        if unsafe { server.poller.add(&listener, Event::readable(0)) }.is_err() {
            sink.reject(call, "could not watch the socket");
            return;
        }
        sink.event(call, json!({ "type": "listening" }), None);
        let mut events = Events::new();
        'serving: while !server.stop.load(Ordering::Relaxed) {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => self.serve_peer(sink, call, server, stream, framing),
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => {
                        sink.event(call, json!({ "type": "error", "message": error.to_string() }), None);
                        break 'serving;
                    }
                }
            }
            events.clear();
            let waited =
                server.poller.modify(&listener, Event::readable(0)).and_then(|_| server.poller.wait(&mut events, None));
            if waited.is_err() {
                break;
            }
        }
        let _ = server.poller.delete(&listener);
        drop(listener);
        if std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_socket()) {
            let _ = std::fs::remove_file(path);
        }
        sink.close(call, json!({}));
    }

    fn serve_peer(&self, sink: &Sink, call: u64, server: &Arc<Server>, stream: UnixStream, framing: Framing) {
        let id = self.last_id.fetch_add(1, Ordering::Relaxed);
        let _ = stream.set_nonblocking(false);
        let (pushed, queued) = channel();
        let Ok(handle) = stream.try_clone() else {
            return;
        };
        let peer = Arc::new(Peer { stream: handle, writing: Arc::default(), pushed });
        locked(&server.peers).insert(id, peer.clone());
        sink.event(call, json!({ "type": "peer", "peer": id }), None);
        if let Ok(writer) = stream.try_clone() {
            let writing = peer.writing.clone();
            std::thread::spawn(move || write_queued(writer, &writing, queued));
        }
        let (unix, sink, server) = (self.clone(), sink.clone(), server.clone());
        std::thread::spawn(move || {
            match framing {
                Framing::Eof => unix.answer_whole(&sink, call, id, &peer),
                Framing::Line => unix.answer_lines(&sink, call, id, &peer),
            }
            let _ = peer.stream.shutdown(Shutdown::Both);
            locked(&server.peers).remove(&id);
            sink.event(call, json!({ "type": "gone", "peer": id }), None);
        });
    }

    /// Reads one request to the end of the peer's input and answers it.
    fn answer_whole(&self, sink: &Sink, call: u64, id: u64, peer: &Peer) {
        let _ = peer.stream.set_read_timeout(Some(PEER_IDLE));
        let mut request = Vec::new();
        let read = (&peer.stream).take(MOST_REQUEST as u64 + 1).read_to_end(&mut request);
        if read.is_err() || request.len() > MOST_REQUEST {
            return;
        }
        self.ask(sink, call, (id, peer), String::from_utf8_lossy(&request).into_owned(), false);
    }

    /// Answers each line the peer sends, until it leaves or an answer fails.
    fn answer_lines(&self, sink: &Sink, call: u64, id: u64, peer: &Peer) {
        let mut splitter = LineSplitter::default();
        let mut buffer = [0u8; 8192];
        loop {
            let count = match (&peer.stream).read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            for line in splitter.push(&buffer[..count]) {
                if !self.ask(sink, call, (id, peer), line, true) {
                    return;
                }
            }
        }
    }

    /// Passes a request to the app and writes its answer. False when the exchange has ended.
    fn ask(&self, sink: &Sink, call: u64, (peer_id, peer): (u64, &Peer), text: String, line: bool) -> bool {
        let id = self.last_id.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = channel();
        locked(&self.pending).insert(id, Pending { server: call, answer });
        sink.event(call, json!({ "type": "request", "id": id, "peer": peer_id, "text": text }), None);
        let Ok(Answer::Data(mut data)) = answered.recv() else {
            return false;
        };
        if line && !data.is_empty() && data.last() != Some(&b'\n') {
            data.push(b'\n');
        }
        let _writing = locked(&peer.writing);
        (&peer.stream).write_all(&data).is_ok()
    }

    /// Hands the app's push to a peer's writer, or ends the peer's connection for a hangup. A
    /// peer that is gone is not an error: the app hears of it as a `gone` event.
    fn push(&self, sink: &Sink, call: u64, args: &Value, output: Output) {
        let server = args["server"].as_u64().and_then(|id| locked(&self.servers).get(&id).cloned());
        let peer = server.zip(args["peer"].as_u64()).and_then(|(server, id)| locked(&server.peers).get(&id).cloned());
        if let Some(peer) = peer {
            let _ = peer.pushed.send(output);
        }
        sink.resolve(call, json!({}));
    }

    fn reply(&self, sink: &Sink, call: u64, args: &Value, bytes: Option<Vec<u8>>) {
        let pending = args["request"].as_u64().and_then(|id| locked(&self.pending).remove(&id));
        if let Some(pending) = pending {
            let failed = args["failed"].as_bool().unwrap_or(false);
            let _ = pending.answer.send(if failed { Answer::Failed } else { Answer::Data(bytes.unwrap_or_default()) });
        }
        sink.resolve(call, json!({}));
    }

    fn stop(&self, sink: &Sink, call: u64, args: &Value) {
        let server = args["server"].as_u64().and_then(|id| locked(&self.servers).get(&id).cloned());
        if let Some(server) = server {
            server.stop.store(true, Ordering::Relaxed);
            let _ = server.poller.notify();
        }
        sink.resolve(call, json!({}));
    }
}

/// Binds the path, first removing a socket file that nothing answers on.
fn bind(raw: &str, path: &Path) -> Result<UnixListener, SocketError> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.file_type().is_socket() {
            return Err(SocketError::new("AddressInUse", format!("{raw} exists and is not a socket")));
        }
        if UnixStream::connect(path).is_ok() {
            return Err(SocketError::new("AddressInUse", format!("{raw} is already served by another process")));
        }
        let _ = std::fs::remove_file(path);
    }
    let listener = UnixListener::bind(path).map_err(|error| SocketError::system("listen on", raw, error))?;
    listener.set_nonblocking(true).map_err(|error| SocketError::system("listen on", raw, error))?;
    Ok(listener)
}

/// Writes what the app queued, in order, and ends the connection when asked. Each write holds
/// `writing`, which others writing to the same stream hold too.
fn write_queued(mut stream: UnixStream, writing: &Mutex<()>, queued: std::sync::mpsc::Receiver<Output>) {
    while let Ok(output) = queued.recv() {
        match output {
            Output::Data(data) => {
                let _writing = locked(writing);
                if stream.write_all(&data).is_err() {
                    break;
                }
            }
            Output::Close => break,
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{Completion, Io, Settle};

    fn unix() -> Unix {
        Unix::new(Directories::find(None))
    }

    fn socket_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("svelte-gpui-{name}-{}.sock", std::process::id()))
    }

    /// Answers every connection with `reply` once the peer has said its piece, on a thread.
    fn echo_server(path: &Path) {
        let listener = UnixListener::bind(path).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut request = Vec::new();
                stream.read_to_end(&mut request).unwrap();
                stream.write_all(b"echo:").unwrap();
                stream.write_all(&request).unwrap();
            }
        });
    }

    #[test]
    fn a_request_reads_the_answer_to_the_end() {
        let path = socket_path("request");
        let _ = std::fs::remove_file(&path);
        echo_server(&path);
        let args = json!({ "path": path.to_string_lossy() });
        let answer = unix().exchange(&args, b"hello".to_vec()).ok().unwrap();
        assert_eq!(answer, b"echo:hello");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_missing_socket_is_not_found() {
        let args = json!({ "path": "/nonexistent/svelte-gpui.sock" });
        let error = unix().exchange(&args, Vec::new()).err().unwrap();
        assert_eq!(error.code, "NotFound");
    }

    #[test]
    fn the_limit_keeps_to_its_roots_and_is_set_once() {
        let unix = unix();
        unix.declare_limit(&json!({ "allow": ["/run/user"] })).ok().unwrap();
        assert!(unix.path(&json!({ "path": "/run/user/1000/hypr/.socket.sock" })).is_ok());
        assert_eq!(unix.path(&json!({ "path": "/tmp/other.sock" })).err().unwrap().code, "NotAllowed");
        assert_eq!(unix.path(&json!({ "path": "/run/user/../../tmp/x" })).err().unwrap().code, "NotAllowed");
        assert_eq!(unix.declare_limit(&json!({ "allow": ["/"] })).err().unwrap().code, "NotAllowed");
    }

    #[test]
    fn a_stale_socket_file_is_replaced_and_a_live_one_is_not() {
        let path = socket_path("stale");
        drop(UnixListener::bind(&path).unwrap());
        let listener = bind("stale", &path);
        assert!(listener.is_ok());
        let live = bind("stale", &path);
        assert_eq!(live.err().unwrap().code, "AddressInUse");
        std::fs::remove_file(&path).unwrap();
    }

    fn next_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<Completion>) -> Completion {
        loop {
            match receiver.try_recv() {
                Ok(completion) => return completion,
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    #[test]
    fn framing_line_answers_each_line() {
        let io = Io::new(None, &[], Default::default());
        let mut receiver = io.take_receiver().unwrap();
        let path = socket_path("line");
        io.request("sock.serve", 1, json!({ "path": path.to_string_lossy(), "framing": "line" }), None);
        assert_eq!(next_event(&mut receiver).json, json!({ "type": "listening" }).to_string());
        let mut client = UnixStream::connect(&path).unwrap();
        client.write_all(b"ping\n").unwrap();
        let peer: Value = serde_json::from_str(&next_event(&mut receiver).json).unwrap();
        assert_eq!(peer["type"], "peer");
        let request = next_event(&mut receiver);
        assert_eq!(request.settle, Settle::Event);
        let request: Value = serde_json::from_str(&request.json).unwrap();
        assert_eq!(request["text"], "ping");
        assert_eq!(request["peer"], peer["peer"]);
        io.request("sock.reply", 2, json!({ "request": request["id"] }), Some(b"pong".to_vec()));
        let mut answer = [0u8; 5];
        client.read_exact(&mut answer).unwrap();
        assert_eq!(&answer, b"pong\n");
        io.request("sock.stop", 3, json!({ "server": 1 }), None);
        loop {
            if next_event(&mut receiver).settle == Settle::Close {
                break;
            }
        }
        assert!(!path.exists());
    }

    #[test]
    fn a_server_can_write_to_a_connected_peer_and_hang_up() {
        let io = Io::new(None, &[], Default::default());
        let mut receiver = io.take_receiver().unwrap();
        let path = socket_path("push");
        io.request("sock.serve", 1, json!({ "path": path.to_string_lossy(), "framing": "line" }), None);
        assert_eq!(next_event(&mut receiver).json, json!({ "type": "listening" }).to_string());
        let mut client = UnixStream::connect(&path).unwrap();
        let peer: Value = serde_json::from_str(&next_event(&mut receiver).json).unwrap();
        let target = json!({ "server": 1, "peer": peer["peer"] });
        io.request("sock.push", 2, target.clone(), Some(b"one\n".to_vec()));
        io.request("sock.push", 3, target.clone(), Some(b"two\n".to_vec()));
        let mut received = [0u8; 8];
        client.read_exact(&mut received).unwrap();
        assert_eq!(&received, b"one\ntwo\n");
        io.request("sock.hangup", 4, target, None);
        let mut rest = Vec::new();
        client.read_to_end(&mut rest).unwrap();
        assert!(rest.is_empty());
        let gone: Value = loop {
            let event: Value = serde_json::from_str(&next_event(&mut receiver).json).unwrap_or_default();
            if event["type"] == "gone" {
                break event;
            }
        };
        assert_eq!(gone["peer"], peer["peer"]);
        io.request("sock.stop", 5, json!({ "server": 1 }), None);
    }
}
