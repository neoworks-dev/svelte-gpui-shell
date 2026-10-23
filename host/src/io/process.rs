//! Processes: run a program to its end, or start one and follow it.
//!
//! Operations, all taking `argv` (the program and its arguments, run without a shell),
//! and optionally `cwd` and `env` (names to values; `null` removes a variable):
//! - `proc.run` `{ argv, timeout? }` runs the program to its end. The request's bytes, if
//!   there are any, are its standard input. It settles once, with `{ status, signal?,
//!   stdoutLength }` and the bytes of standard output followed by those of standard error.
//! - `proc.spawn` `{ argv }` starts a program and is a stream: the events `{ type:
//!   "stdout" | "stderr", line }`, one per line as the program writes it, and `close`
//!   with `{ status, signal? }` once it has exited and its output is read.
//! - `proc.write` `{ process }` (bytes), `proc.closeStdin` `{ process }` and `proc.kill`
//!   `{ process, signal }` act on the stream that `proc.spawn` opened.
//! - `proc.env` answers with the environment the host runs in.
//! - `proc.limit` `{ allow }` limits which programs may run, once.
//!
//! Every process runs in a group of its own, so signals reach what it started too, and
//! the system kills it with the host should the host die without cleaning up. A process
//! that exits by a signal reports `status` as 128 plus the signal, the way a shell does.
//!
//! `kill` never reaches a process that has been reaped: the thread that waits for the exit
//! looks without reaping, marks the process as gone, and only then reaps it, so the number
//! cannot have been handed to another process in between.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use super::Sink;
use super::lines::for_each_line;

/// How long output still held open by something the process started (a daemon that
/// inherited the pipe) is waited for once the process itself has exited.
const OUTPUT_GRACE: Duration = Duration::from_millis(300);

/// Why a process call failed: a code the app can tell apart, and a message for people.
struct ProcessError {
    code: &'static str,
    message: String,
}

impl ProcessError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        ProcessError { code, message: message.into() }
    }

    fn argument(message: impl Into<String>) -> Self {
        ProcessError::new("InvalidArgument", message)
    }

    fn start(program: &str, error: std::io::Error) -> Self {
        let code = match error.kind() {
            ErrorKind::NotFound => "NotFound",
            ErrorKind::PermissionDenied => "PermissionDenied",
            _ => "Other",
        };
        ProcessError::new(code, format!("cannot run {program}: {error}"))
    }
}

/// What the app asked to run.
struct Spec {
    argv: Vec<String>,
    cwd: Option<String>,
    env: Vec<(String, Option<String>)>,
}

impl Spec {
    fn parse(args: &Value) -> Result<Spec, ProcessError> {
        let argv: Vec<String> = args["argv"]
            .as_array()
            .ok_or_else(|| ProcessError::argument("argv has to be a list of strings"))?
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect::<Option<_>>()
            .ok_or_else(|| ProcessError::argument("argv has to be a list of strings"))?;
        if argv.first().is_none_or(|program| program.is_empty()) {
            return Err(ProcessError::argument("argv needs a program to run"));
        }
        let env = match args["env"].as_object() {
            Some(map) => map.iter().map(|(name, value)| (name.clone(), value.as_str().map(str::to_string))).collect(),
            None => Vec::new(),
        };
        Ok(Spec { argv, cwd: args["cwd"].as_str().map(str::to_string), env })
    }
}

enum State {
    /// Not started yet; a signal that arrives now is sent as soon as it has.
    Starting(Option<i32>),
    Running(i32),
    Exited,
}

enum Input {
    Data(Vec<u8>),
    Close,
}

/// One process, as seen from the app's requests.
struct Process {
    state: Mutex<State>,
    input: Mutex<Option<Sender<Input>>>,
}

impl Process {
    fn new() -> Arc<Process> {
        Arc::new(Process { state: Mutex::new(State::Starting(None)), input: Mutex::new(None) })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn started(&self, pid: i32) {
        let mut state = self.state();
        if let State::Starting(Some(signal)) = *state {
            send_signal(pid, signal);
        }
        *state = State::Running(pid);
    }

    fn exited(&self) {
        *self.state() = State::Exited;
    }

    /// Signals the process and everything in its group.
    fn signal(&self, signal: i32) {
        let mut state = self.state();
        match &mut *state {
            State::Starting(pending) => *pending = Some(signal),
            State::Running(pid) => send_signal(*pid, signal),
            State::Exited => {}
        }
    }

    fn send(&self, input: Input) {
        let input_sender = self.input.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = input_sender.as_ref() {
            let _ = sender.send(input);
        }
    }
}

fn send_signal(pid: i32, signal: i32) {
    // Safety: a plain system call; the group was made by this process, and see the module
    // docs for why the number is still ours.
    unsafe { libc::kill(-pid, signal) };
}

#[derive(Clone, Default)]
pub struct Processes {
    /// The programs the app allowed, once it has declared a limit.
    allowed: Arc<Mutex<Option<Vec<String>>>>,
    running: Arc<Mutex<HashMap<u64, Arc<Process>>>>,
}

impl Processes {
    pub fn request(&self, sink: &Sink, op: &str, call: u64, args: Value, bytes: Option<Vec<u8>>) {
        match op {
            "proc.run" => self.run(sink, call, args, bytes),
            "proc.spawn" => self.spawn(sink, call, args),
            "proc.write" => self.act(sink, call, &args, Input::Data(bytes.unwrap_or_default())),
            "proc.closeStdin" => self.act(sink, call, &args, Input::Close),
            "proc.kill" => self.kill(sink, call, &args),
            "proc.limit" => self.limit(sink, call, &args),
            "proc.env" => {
                let env: serde_json::Map<String, Value> = std::env::vars_os()
                    .map(|(name, value)| (name.to_string_lossy().into_owned(), json!(value.to_string_lossy())))
                    .collect();
                sink.resolve(call, json!({ "env": env }));
            }
            _ => sink.reject(call, format!("svelte-gpui: unknown operation {op}")),
        }
    }

    /// Kills every process still running, for an app that is going away.
    pub fn kill_all(&self) {
        for process in self.running().values() {
            process.signal(libc::SIGKILL);
        }
    }

    fn running(&self) -> MutexGuard<'_, HashMap<u64, Arc<Process>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn limit(&self, sink: &Sink, call: u64, args: &Value) {
        let mut entries = Vec::new();
        for entry in args["allow"].as_array().into_iter().flatten() {
            match entry.as_str() {
                Some(program) if program.starts_with('/') || !program.contains('/') => {
                    entries.push(program.to_string())
                }
                _ => {
                    let message = "allow holds program names, or absolute paths";
                    sink.reject_with_code(call, "InvalidArgument", message);
                    return;
                }
            }
        }
        let mut allowed = self.allowed.lock().unwrap_or_else(PoisonError::into_inner);
        if allowed.is_some() {
            let message = "the process limit is already set and cannot be changed";
            sink.reject_with_code(call, "NotAllowed", message);
            return;
        }
        *allowed = Some(entries);
        sink.resolve(call, json!({}));
    }

    /// Fails unless the limit, if the app set one, allows `program`. A name is looked up in
    /// `PATH`, and what it finds is compared with what each allowed entry finds, so `nmcli`
    /// and `/usr/bin/nmcli` are the same program. Links are not followed: allowing `sh`
    /// does not allow whatever it points to.
    fn check(&self, program: &str) -> Result<(), ProcessError> {
        let allowed = self.allowed.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(entries) = allowed.as_ref() else {
            return Ok(());
        };
        let search = std::env::var_os("PATH").unwrap_or_default();
        let Some(path) = resolve_program(program, &search) else {
            if program.contains('/') {
                return Err(ProcessError::new("NotAllowed", format!("running {program} is not allowed")));
            }
            return Err(ProcessError::new("NotFound", format!("cannot run {program}: not found in PATH")));
        };
        let permitted = entries.iter().any(|entry| resolve_program(entry, &search).as_deref() == Some(path.as_path()));
        if permitted {
            return Ok(());
        }
        Err(ProcessError::new("NotAllowed", format!("running {program} is not allowed")))
    }

    fn act(&self, sink: &Sink, call: u64, args: &Value, input: Input) {
        let process = args["process"].as_u64().and_then(|id| self.running().get(&id).cloned());
        if let Some(process) = process {
            process.send(input);
        }
        sink.resolve(call, json!({}));
    }

    fn kill(&self, sink: &Sink, call: u64, args: &Value) {
        let Some(signal) = parse_signal(&args["signal"]) else {
            sink.reject_with_code(call, "InvalidArgument", format!("unknown signal {}", args["signal"]));
            return;
        };
        let process = args["process"].as_u64().and_then(|id| self.running().get(&id).cloned());
        if let Some(process) = process {
            process.signal(signal);
        }
        sink.resolve(call, json!({}));
    }

    fn run(&self, sink: &Sink, call: u64, args: Value, stdin: Option<Vec<u8>>) {
        let (processes, sink) = (self.clone(), sink.clone());
        std::thread::spawn(move || match processes.run_to_end(call, &args, stdin) {
            Ok((json, bytes)) => sink.resolve_bytes(call, json, bytes),
            Err(error) => sink.reject_with_code(call, error.code, error.message),
        });
    }

    fn run_to_end(&self, call: u64, args: &Value, stdin: Option<Vec<u8>>) -> Result<(Value, Vec<u8>), ProcessError> {
        let spec = Spec::parse(args)?;
        self.check(&spec.argv[0])?;
        let process = Process::new();
        let mut child = launch(&spec, stdin.is_some(), &process)?;
        self.running().insert(call, process.clone());
        let writer = child.stdin.take().zip(stdin).map(|(mut pipe, data)| {
            std::thread::spawn(move || {
                let _ = pipe.write_all(&data);
            })
        });
        let out = child.stdout.take().map(|pipe| std::thread::spawn(move || read_all(pipe)));
        let err = child.stderr.take().map(|pipe| std::thread::spawn(move || read_all(pipe)));
        let timed_out = Arc::new(AtomicBool::new(false));
        let (finished, watch) = channel::<()>();
        let timeout = args["timeout"].as_u64().map(Duration::from_millis);
        if let Some(limit) = timeout {
            let (process, timed_out) = (process.clone(), timed_out.clone());
            std::thread::spawn(move || {
                if watch.recv_timeout(limit) == Err(RecvTimeoutError::Timeout) {
                    timed_out.store(true, Ordering::Relaxed);
                    process.signal(libc::SIGKILL);
                }
            });
        }
        let status = wait_exit(&mut child, &process);
        drop(finished);
        let mut stdout = out.and_then(|reader| reader.join().ok()).unwrap_or_default();
        let stderr = err.and_then(|reader| reader.join().ok()).unwrap_or_default();
        drop(writer);
        self.running().remove(&call);
        if timed_out.load(Ordering::Relaxed) {
            let milliseconds = timeout.map(|limit| limit.as_millis()).unwrap_or_default();
            return Err(ProcessError::new("TimedOut", format!("{} timed out after {milliseconds} ms", spec.argv[0])));
        }
        let status =
            status.map_err(|error| ProcessError::new("Other", format!("cannot wait for the process: {error}")))?;
        let mut json = describe(&status);
        json["stdoutLength"] = json!(stdout.len());
        stdout.extend_from_slice(&stderr);
        Ok((json, stdout))
    }

    fn spawn(&self, sink: &Sink, call: u64, args: Value) {
        let process = Process::new();
        let (sender, receiver) = channel();
        *process.input.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
        self.running().insert(call, process.clone());
        let (processes, sink) = (self.clone(), sink.clone());
        std::thread::spawn(move || {
            let outcome = Spec::parse(&args)
                .and_then(|spec| processes.check(&spec.argv[0]).map(|_| spec))
                .and_then(|spec| launch(&spec, true, &process));
            match outcome {
                Ok(child) => follow(&sink, call, child, &process, receiver),
                Err(error) => sink.reject_with_code(call, error.code, error.message),
            }
            processes.running().remove(&call);
        });
    }
}

/// Starts the program. Standard input is a pipe when `piped_input`, and closed otherwise.
fn launch(spec: &Spec, piped_input: bool, process: &Process) -> Result<Child, ProcessError> {
    let mut command = Command::new(&spec.argv[0]);
    command
        .args(&spec.argv[1..])
        .stdin(if piped_input { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(directory) = &spec.cwd {
        command.current_dir(directory);
    }
    for (name, value) in &spec.env {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
    #[cfg(target_os = "linux")]
    // Safety: only a system call between fork and exec. The signal fires when the thread
    // that started the process ends, which is the thread that waits for it, or the host.
    unsafe {
        command.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            Ok(())
        });
    }
    let child = command.spawn().map_err(|error| ProcessError::start(&spec.argv[0], error))?;
    process.started(child.id() as i32);
    Ok(child)
}

/// Streams a process's lines until it has exited, then closes the call.
fn follow(sink: &Sink, call: u64, mut child: Child, process: &Process, input: Receiver<Input>) {
    if let Some(mut pipe) = child.stdin.take() {
        std::thread::spawn(move || {
            while let Ok(Input::Data(data)) = input.recv() {
                if pipe.write_all(&data).is_err() {
                    break;
                }
            }
        });
    }
    let (finished, outputs) = channel::<()>();
    let mut open = 0;
    let pipes: [(&str, Option<Box<dyn Read + Send>>); 2] = [
        ("stdout", child.stdout.take().map(|pipe| Box::new(pipe) as Box<dyn Read + Send>)),
        ("stderr", child.stderr.take().map(|pipe| Box::new(pipe) as Box<dyn Read + Send>)),
    ];
    for (kind, pipe) in pipes {
        let Some(pipe) = pipe else { continue };
        open += 1;
        let (sink, finished) = (sink.clone(), finished.clone());
        std::thread::spawn(move || {
            for_each_line(pipe, |line| sink.event(call, json!({ "type": kind, "line": line }), None));
            let _ = finished.send(());
        });
    }
    let status = wait_exit(&mut child, process);
    for _ in 0..open {
        if outputs.recv_timeout(OUTPUT_GRACE).is_err() {
            break;
        }
    }
    match status {
        Ok(status) => sink.close(call, describe(&status)),
        Err(error) => sink.close(call, json!({ "status": -1, "error": error.to_string() })),
    }
}

fn read_all(mut pipe: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    bytes
}

/// Waits for the process to exit. See the module docs for why this does not reap first.
fn wait_exit(child: &mut Child, process: &Process) -> std::io::Result<ExitStatus> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    loop {
        // Safety: `info` is a valid, writable siginfo_t.
        let result = unsafe { libc::waitid(libc::P_PID, child.id(), &mut info, libc::WEXITED | libc::WNOWAIT) };
        let interrupted = std::io::Error::last_os_error().kind() == ErrorKind::Interrupted;
        if result == 0 || !interrupted {
            break;
        }
    }
    process.exited();
    child.wait()
}

fn describe(status: &ExitStatus) -> Value {
    match status.signal() {
        Some(signal) => json!({ "status": 128 + signal, "signal": signal_name(signal) }),
        None => json!({ "status": status.code().unwrap_or(-1) }),
    }
}

const SIGNALS: [(i32, &str); 13] = [
    (libc::SIGHUP, "SIGHUP"),
    (libc::SIGINT, "SIGINT"),
    (libc::SIGQUIT, "SIGQUIT"),
    (libc::SIGABRT, "SIGABRT"),
    (libc::SIGKILL, "SIGKILL"),
    (libc::SIGUSR1, "SIGUSR1"),
    (libc::SIGUSR2, "SIGUSR2"),
    (libc::SIGPIPE, "SIGPIPE"),
    (libc::SIGALRM, "SIGALRM"),
    (libc::SIGTERM, "SIGTERM"),
    (libc::SIGCONT, "SIGCONT"),
    (libc::SIGSTOP, "SIGSTOP"),
    (libc::SIGSEGV, "SIGSEGV"),
];

fn signal_name(signal: i32) -> String {
    match SIGNALS.iter().find(|(number, _)| *number == signal) {
        Some((_, name)) => name.to_string(),
        None => format!("SIG{signal}"),
    }
}

/// The signal an app named: a name such as `SIGTERM`, or a number. Absent means `SIGTERM`.
fn parse_signal(value: &Value) -> Option<i32> {
    match value {
        Value::Null => Some(libc::SIGTERM),
        Value::String(name) => SIGNALS.iter().find(|(_, known)| known == name).map(|(number, _)| *number),
        Value::Number(number) => number.as_i64().filter(|number| (1..=64).contains(number)).map(|number| number as i32),
        _ => None,
    }
}

/// The absolute path a program name stands for. A name is looked up in `search`, a `PATH`
/// value; a path has to be absolute.
fn resolve_program(program: &str, search: &OsStr) -> Option<PathBuf> {
    if program.contains('/') {
        return Path::new(program).is_absolute().then(|| PathBuf::from(program));
    }
    std::env::split_paths(search).map(|directory| directory.join(program)).find(|candidate| {
        std::fs::metadata(candidate).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{Completion, Io, Settle};
    use std::os::unix::fs::PermissionsExt;

    fn program_in(directory: &Path, name: &str, mode: u32) {
        let path = directory.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn a_name_is_found_in_the_search_path_and_a_path_has_to_be_absolute() {
        let directory = std::env::temp_dir().join(format!("svelte-gpui-process-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        program_in(&directory, "runnable", 0o755);
        program_in(&directory, "plain", 0o644);
        let search = std::env::join_paths([&directory]).unwrap();
        assert_eq!(resolve_program("runnable", &search), Some(directory.join("runnable")));
        assert_eq!(resolve_program("plain", &search), None);
        assert_eq!(resolve_program("missing", &search), None);
        assert_eq!(resolve_program("/bin/sh", &search), Some(PathBuf::from("/bin/sh")));
        assert_eq!(resolve_program("./runnable", &search), None);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn a_signal_exit_reports_the_shell_status_and_name() {
        let status = ExitStatus::from_raw(libc::SIGTERM);
        assert_eq!(describe(&status), json!({ "status": 143, "signal": "SIGTERM" }));
        assert_eq!(describe(&ExitStatus::from_raw(3 << 8)), json!({ "status": 3 }));
    }

    #[test]
    fn signals_are_named_or_numbered() {
        assert_eq!(parse_signal(&json!("SIGKILL")), Some(libc::SIGKILL));
        assert_eq!(parse_signal(&json!(2)), Some(2));
        assert_eq!(parse_signal(&Value::Null), Some(libc::SIGTERM));
        assert_eq!(parse_signal(&json!("KILL")), None);
        assert_eq!(parse_signal(&json!(0)), None);
    }

    #[test]
    fn the_limit_matches_by_resolved_program() {
        let processes = Processes::default();
        *processes.allowed.lock().unwrap() = Some(vec!["/bin/sh".to_string()]);
        assert!(processes.check("/bin/sh").is_ok());
        assert_eq!(processes.check("/bin/cat").err().map(|error| error.code), Some("NotAllowed"));
        assert_eq!(processes.check("./sh").err().map(|error| error.code), Some("NotAllowed"));
    }

    /// Completions of `call` up to and including the one that settles or ends it.
    fn until_settled(
        receiver: &mut futures::channel::mpsc::UnboundedReceiver<Completion>,
        call: u64,
    ) -> Vec<Completion> {
        let mut seen = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            let Ok(completion) = receiver.try_recv() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            let last = completion.call == call && !matches!(completion.settle, Settle::Event);
            seen.push(completion);
            if last {
                return seen;
            }
        }
        panic!("call {call} did not settle");
    }

    fn io() -> (Io, futures::channel::mpsc::UnboundedReceiver<Completion>) {
        let io = Io::new(None, &[], Default::default());
        let receiver = io.take_receiver().unwrap();
        (io, receiver)
    }

    #[test]
    fn run_collects_both_outputs_and_the_status() {
        let (io, mut receiver) = io();
        io.request("proc.run", 1, json!({ "argv": ["sh", "-c", "printf out; printf err >&2; exit 3"] }), None);
        let done = until_settled(&mut receiver, 1).pop().unwrap();
        assert_eq!(done.json, json!({ "status": 3, "stdoutLength": 3 }).to_string());
        assert_eq!(done.bytes.unwrap(), b"outerr");
    }

    #[test]
    fn run_feeds_standard_input() {
        let (io, mut receiver) = io();
        io.request("proc.run", 1, json!({ "argv": ["cat"] }), Some(b"typed".to_vec()));
        let done = until_settled(&mut receiver, 1).pop().unwrap();
        assert_eq!(done.bytes.unwrap(), b"typed");
    }

    #[test]
    fn run_rejects_a_program_that_is_not_there_and_one_that_times_out() {
        let (io, mut receiver) = io();
        io.request("proc.run", 1, json!({ "argv": ["svelte-gpui-no-such-program"] }), None);
        let done = until_settled(&mut receiver, 1).pop().unwrap();
        assert_eq!(done.settle, Settle::Reject);
        assert!(done.json.contains("NotFound"), "{}", done.json);
        io.request("proc.run", 2, json!({ "argv": ["sleep", "5"], "timeout": 100 }), None);
        let done = until_settled(&mut receiver, 2).pop().unwrap();
        assert!(done.json.contains("TimedOut"), "{}", done.json);
    }

    #[test]
    fn spawn_streams_lines_and_reports_the_exit() {
        let (io, mut receiver) = io();
        let script = "echo one; echo two >&2; sleep 0.1; echo three; exit 2";
        io.request("proc.spawn", 1, json!({ "argv": ["sh", "-c", script] }), None);
        let seen = until_settled(&mut receiver, 1);
        let events: Vec<Value> =
            seen.iter().map(|completion| serde_json::from_str(&completion.json).unwrap()).collect();
        for expected in [("stdout", "one"), ("stderr", "two"), ("stdout", "three")] {
            let event = json!({ "type": expected.0, "line": expected.1 });
            assert!(events.contains(&event), "{events:?}");
        }
        assert_eq!(events.last().unwrap(), &json!({ "status": 2 }));
    }

    #[test]
    fn spawn_takes_input_until_it_is_closed_and_can_be_killed() {
        let (io, mut receiver) = io();
        io.request("proc.spawn", 1, json!({ "argv": ["cat"] }), None);
        io.request("proc.write", 2, json!({ "process": 1 }), Some(b"hello\n".to_vec()));
        io.request("proc.closeStdin", 3, json!({ "process": 1 }), None);
        let seen = until_settled(&mut receiver, 1);
        assert!(seen.iter().any(|completion| completion.json.contains("hello")));
        io.request("proc.spawn", 4, json!({ "argv": ["sleep", "30"] }), None);
        std::thread::sleep(Duration::from_millis(100));
        io.request("proc.kill", 5, json!({ "process": 4, "signal": "SIGTERM" }), None);
        let done = until_settled(&mut receiver, 4).pop().unwrap();
        assert_eq!(done.json, json!({ "status": 143, "signal": "SIGTERM" }).to_string());
    }

    #[test]
    fn dropping_the_io_kills_what_is_running() {
        let (io, _receiver) = io();
        io.request(
            "proc.spawn",
            1,
            json!({ "argv": ["sh", "-c", "echo $$ > /tmp/svelte-gpui-drop.pid; sleep 30"] }),
            None,
        );
        let pid_file = "/tmp/svelte-gpui-drop.pid";
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::fs::read_to_string(pid_file).map_or(true, |text| text.trim().is_empty()) {
            assert!(std::time::Instant::now() < deadline, "the process never started");
            std::thread::sleep(Duration::from_millis(10));
        }
        let pid: i32 = std::fs::read_to_string(pid_file).unwrap().trim().parse().unwrap();
        drop(io);
        std::thread::sleep(Duration::from_millis(300));
        // Safety: signal 0 only checks that the process exists.
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        let zombie = std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| stat.contains(") Z"));
        assert!(!alive || zombie, "process {pid} survived");
        let _ = std::fs::remove_file(pid_file);
    }
}
