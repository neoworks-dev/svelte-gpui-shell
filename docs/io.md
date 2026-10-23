# Network and files

An app reaches the outside world through these APIs:

- **`fetch`**, the web's own.
- **`WebSocket`**, the web's own.
- **`files`**, from `@neoworks/svelte-gpui`.
- **`processes`**, from `@neoworks/svelte-gpui`: run programs and follow their output.
- **`unixSockets`**, from `@neoworks/svelte-gpui`: talk to local daemons, and be one.

The host runs each request on a background thread, and the result settles the promise
or fires the event as soon as it arrives, so the window never waits on the network or
the disk.

`examples/demo` has a Network section and a Files section that use the first three. `examples/native-io`
checks `processes` and `unixSockets` against the real system.

## fetch

`fetch` works as in a browser, so code and libraries written for the web work unchanged:

```ts
const response = await fetch("https://api.github.com/repos/zed-industries/zed", {
  headers: { accept: "application/vnd.github+json" },
});
if (!response.ok) {
  throw new Error(`GitHub said ${response.status}`);
}
const repository = await response.json();
```

| Part | Supported |
|---|---|
| Input | A URL string, a `URL`, or a `Request`. |
| `method`, `headers` | Any. Headers can be an object, a list of pairs, or a `Headers`. |
| `body` | A string, `ArrayBuffer`, typed array, `DataView` or `URLSearchParams`. Strings get `text/plain;charset=UTF-8` and `URLSearchParams` gets `application/x-www-form-urlencoded`, unless you set a content type. |
| `redirect` | `"follow"` (the default, up to 20 hops), `"manual"` or `"error"`. |
| `signal` | An `AbortSignal`, including `AbortSignal.timeout(ms)` and `AbortSignal.any([...])`. Aborting rejects with an `AbortError`. |
| `Response` | `ok`, `status`, `statusText`, `headers`, `url`, `redirected`, `text()`, `json()`, `arrayBuffer()`, `bytes()`, `clone()`, `bodyUsed`. |

As in browsers, a 404 or 500 **resolves** with that status. Only a failure to reach
the server (DNS, a refused connection, TLS) rejects, with a `TypeError`.

`Headers`, `Request`, `Response`, `AbortController`, `AbortSignal`, `TextEncoder`,
`TextDecoder` (UTF-8) and `DOMException` are globals too.

Not supported:
- Bodies are read whole before the promise resolves, so there is no `response.body`
  stream and no upload progress.
- There is no `Blob` or `FormData`.
- Cookies are not kept between requests.
- An aborted request still finishes on its thread; the result is dropped.

## WebSocket

```ts
const socket = new WebSocket("wss://example.com/live");
socket.binaryType = "arraybuffer";
socket.onopen = () => socket.send(JSON.stringify({ subscribe: "prices" }));
socket.onmessage = (event) => update(JSON.parse(event.data));
socket.onclose = (event) => console.log("closed", event.code, event.reason);
```

Each socket has its own thread, which sleeps until a message arrives or the app sends
one. An idle socket costs nothing, and a message goes out at once.

| Part | Supported |
|---|---|
| `new WebSocket(url, protocols?)` | `ws://`, `wss://`, and `http(s)://`, which mean the same. |
| Events | `onopen`, `onmessage`, `onerror` and `onclose`, plus `addEventListener` and `removeEventListener`. |
| `send(data)` | A string, `ArrayBuffer`, typed array or `DataView`. It throws `InvalidStateError` before `open`. |
| `close(code?, reason?)` | Waits up to 5 seconds for the server's reply. |
| `MessageEvent.data` | A string, or an `ArrayBuffer` for binary messages. |
| `CloseEvent` | `code`, `reason`, `wasClean`. |
| Properties | `readyState` and its constants, `protocol`, `url`, `binaryType`. |

`binaryType` can only be `"arraybuffer"`, because there is no `Blob`. A connection that
fails fires `error` and then `close` with code 1006, as in browsers. `bufferedAmount`
is always 0.

## Files

```ts
import { files, FileError } from "@neoworks/svelte-gpui";

const { data } = await files.paths();
const settingsPath = `${data}/my-app/settings.json`;

await files.mkdir(`${data}/my-app`, { recursive: true });
await files.writeText(settingsPath, JSON.stringify(settings));

try {
  settings = JSON.parse(await files.readText(settingsPath));
} catch (error) {
  if (!(error instanceof FileError && error.code === "NotFound")) throw error;
}
```

Every call returns a promise and runs on a thread.

| Function | Result |
|---|---|
| `readText(path)` | `string`. Rejects with `InvalidData` if the file is not UTF-8. |
| `readBytes(path)` | `Uint8Array` |
| `writeText(path, text)`, `writeBytes(path, data)` | Creates or replaces the file. |
| `appendText(path, text)` | Creates the file if needed. |
| `exists(path)` | `boolean` |
| `stat(path)` | `{ kind, size, modified, created? }`. `kind` is `"file"`, `"directory"` or `"symlink"`, and times are ms since the epoch. A final symlink is not followed. |
| `list(directory)` | Entries sorted by name, each `{ name, path, kind, size, modified, created? }`. |
| `mkdir(path, { recursive? })` | |
| `remove(path, { recursive? })` | A directory needs `recursive` unless it is empty. A symlink removes the link, not its target. |
| `rename(from, to)`, `copy(from, to)` | `copy` copies directories recursively. `rename` does not work across file systems. |
| `watch(path, callback, { recursive?, onerror? })` | Calls `callback({ kind, path })` with `kind` `"create"`, `"modify"` or `"remove"`, as changes happen. Returns a function that stops the watch. |
| `paths()` | `{ home, config, data, cache, temp, runtime, bundle, cwd }`: the system's directories (XDG on Linux, with `runtime` the session's `XDG_RUNTIME_DIR`), the bundle's directory, and the working directory. |
| `pickFiles({ multiple?, directories?, prompt? })` | The system's open dialog. Resolves with `string[]`, or `null` if cancelled. |
| `pickSavePath({ directory?, name? })` | The system's save dialog. Resolves with a `string`, or `null`. |

Paths:
- `~` at the start of a path means the home directory.
- A relative path resolves against the directory the process started in. An app cannot
  rely on what that is, so build absolute paths from `paths()`.

Failures reject with a `FileError`:
- `message` names the operation, the path and the system's reason.
- `code` is one of `NotFound`, `PermissionDenied`, `NotAllowed`, `Exists`, `NotEmpty`,
  `NotADirectory`, `IsADirectory`, `InvalidData`, `InvalidArgument` or `Other`.

Watches use the system's change notifications (inotify on Linux), so they report at
once and cost nothing while idle. A rename shows as a `remove` and a `create`. A watch
that cannot start (for instance on a missing path) calls `onerror`, or logs if there is
none.

### Limiting file access

By default an app may read and write anything the user can, like any native program.
An app that only needs some places can say so when it starts, and the host then
refuses everything else:

```ts
start(App, {}, {
  files: { allow: ["$data/my-app", "$config/my-app", "~/Documents/My App"] },
});
```

- **What an entry grants:** each entry allows itself and everything below it.
- **Placeholders:** `$home`, `$config`, `$data`, `$cache`, `$temp`, `$runtime`, `$bundle` and `$cwd`
  at the start of an entry stand for the directories from `files.paths()`, and `~` for
  home.
- **Nothing:** `allow: []` allows no files at all.
- **Dialogs:** whatever the user picks in `pickFiles` or `pickSavePath` is allowed from
  then on, even outside the limit. A picked directory allows everything below it.
- **Dropped files:** files dragged onto the window from another program and dropped on an element
  that listens for `ondrop` are allowed the same way, so a drop area can read what was dropped (see
  [Events](events.md#files-dragged-in)). Nothing is allowed for a drop no element takes.
- **Set once:** the limit is set before the app mounts and cannot be changed or widened
  later, so code that runs afterwards (including a dependency) cannot lift it.
- **How it is checked:** on the real path, after following symlinks and resolving `..`,
  so neither can escape. Both sides of `rename` and `copy` are checked.
- **What a refusal looks like:** a refused call rejects with code `NotAllowed`. A refused
  watch calls `onerror`.

Whoever runs the app can narrow it further with the host flag `--allow-files PATH`,
which can be repeated. When both the app and the flag set limits, a path must be inside
both, so the flag can only take access away. `paths()` and the dialogs always work.

## Processes

```ts
import { processes } from "@neoworks/svelte-gpui";

const { status, stdout } = await processes.run(["nmcli", "-t", "-f", "SSID,SIGNAL", "device", "wifi"]);

const subscription = processes.spawn(["pactl", "subscribe"], {
  onStdout: (line) => refreshVolume(line),
  onExit: ({ status }) => console.log("pactl ended", status),
});
// later
subscription.kill();
```

A program is run from its argument list, with no shell: `["sh", "-c", "…"]` is how to ask for
one. The first entry is a name looked up in `PATH`, or a path. Each call runs on a thread.

| Function | Result |
|---|---|
| `run(argv, { stdin?, cwd?, env?, timeout?, encoding? })` | `{ status, signal?, stdout, stderr }` once the program has exited. With `encoding: "bytes"`, `stdout` is a `Uint8Array` of exactly what the program wrote, and not decoded text. `stdin` is text or bytes, sent and then closed; without it the program's input is closed at once. Output is read as UTF-8. A program that exits with a failure status still resolves, as a 404 does for `fetch`. With `timeout` (ms), a program that runs longer is killed and the call rejects with `TimedOut`. |
| `spawn(argv, { cwd?, env?, onStdout?, onStdoutBytes?, onStderr?, onExit?, onError? })` | A handle, at once. |
| `env()` | `Record<string, string>`: the host's environment, which programs inherit. |

`env` in the options lays variables over the inherited ones, and `null` removes one.

Binary output, such as a raw frame from a capture tool, is read with `run(argv, { encoding: "bytes" })`
or, as it streams, with `onStdoutBytes(chunk)` in place of `onStdout`: the chunks are what the
program wrote, not split at line ends. Standard error stays text.

A handle from `spawn` has:
- `onStdout(line)` and `onStderr(line)`, called for each line as the program writes it,
  without the line ending (`\n` or `\r\n`). The two streams are separate, so their
  order relative to each other is not kept. A line longer than 1 MiB arrives in pieces, and
  a last line without a newline arrives when the program ends.
- `onExit({ status, signal? })`, called after the program has exited and its output has been
  delivered, and `exited`, a promise of the same. A program that a signal ended has `status` of
  128 plus the signal, as in a shell, and `signal` set to its name.
- `write(data)`, which sends text or bytes to the program's input and throws if input is closed or
  the program has exited; `closeStdin()`, which most programs take as the end of their work.
- `kill(signal?)`, which sends a signal (`"SIGTERM"` by default, or a name such as `"SIGKILL"`, or a
  number) to the program and to everything it started, since each program has a process group of its own.

A program that cannot be started rejects `run`, rejects `exited`, and calls `onError` (or
logs, if there is none). Failures are a `ProcessError` with a `code`: `NotFound`,
`PermissionDenied`, `NotAllowed`, `InvalidArgument`, `TimedOut` or `Other`.

Programs still running when the app quits or reloads (`--watch`) are killed, and so are
they if the host itself dies.

### Limiting programs

By default an app may run anything the user can. An app that needs a few programs can say so,
once, before it mounts:

```ts
start(App, {}, { process: { allow: ["nmcli", "bluetoothctl", "/usr/bin/unix_chkpwd"] } });
```

An entry is a program name, looked up in `PATH`, or an absolute path, and a program is allowed when
it is the same file as an entry: `nmcli` and `/usr/bin/nmcli` are one. Links are not followed, so
allowing `sh` does not allow what it points to. A relative path with a slash is refused. Refused
calls reject with `NotAllowed`. There is no host flag to narrow it, and an allowed interpreter (a
shell, `env`, `python`) can run anything, so allow the programs, not a way to start them. The
environment can always be read with `env()`.

## Unix sockets

```ts
import { files, processes, unixSockets } from "@neoworks/svelte-gpui";

const { runtime } = await files.paths();
const env = await processes.env();
const hypr = `${runtime}/hypr/${env.HYPRLAND_INSTANCE_SIGNATURE}`;

const monitors = JSON.parse(await unixSockets.request(`${hypr}/.socket.sock`, "j/monitors"));

unixSockets.connect(`${hypr}/.socket2.sock`, { onLine: (line) => handleEvent(line) });

const server = unixSockets.serve(`${runtime}/my-app.sock`, async (request) => {
  return JSON.stringify(await handle(request));
});
```

| Function | Result |
|---|---|
| `request(path, data, { timeout?, halfClose? })` | Connects, sends `data` (text or bytes), reads until the server closes, and resolves with what it sent as text. `timeout` is in ms and defaults to 30000. After sending, the writing end is closed so the server sees the request end; `halfClose: false` leaves it open for servers that dislike that. |
| `requestBytes(path, data, options)` | The same, resolving with a `Uint8Array`. |
| `connect(path, { onOpen?, onLine?, onClose?, onError? })` | A connection handle, at once. `onLine(line)` is called per line as it arrives, split as for processes. |
| `serve(path, handler, { framing?, onError? })` | A server handle, at once. |

A connection handle has `write(data)` (queued, and sent in order once connected; throws after the
connection has ended), `close()` (after what was queued is sent) and `closed`, a promise that settles
when the connection ends, or rejects if it could not be made.

A server listens on `path`, first removing a socket file that nothing answers on, as a crashed run
leaves behind. A path that is a live socket of another process, or is not a socket, rejects `ready`
with `AddressInUse`. The handler gets each request's text and returns the answer, as text or bytes,
or a promise of it; returning nothing writes nothing. A handler that throws ends that connection
without an answer and calls `onError`. `framing` says what a request is:
- `"eof"` (the default): everything the peer sends until it closes its writing end, up to 16 MiB. The
  answer ends the connection. This is what `request` speaks, and what Hyprland does.
- `"line"`: every line is a request, answered in order with a line (a newline is added if the answer
  lacks one), and the connection stays open until the peer leaves. `nc -U` and `socat` work with it.

A server handle has `path`, `ready` (a promise that settles once the server listens) and `close()`,
which stops listening, ends its connections and removes the socket file. Servers and connections left
open when the app quits or reloads are closed, and their files removed.

Failures are a `SocketError` with a `code`: `NotFound`, `PermissionDenied`, `NotAllowed`,
`ConnectionRefused`, `AddressInUse`, `TimedOut`, `InvalidArgument` or `Other`.

### Limiting sockets

By default an app may use any socket path the user can. To limit it, once, before it mounts:

```ts
start(App, {}, { sockets: { allow: ["$runtime/hypr", "$runtime/my-app.sock"] } });
```

The entries work as for [files](#limiting-file-access): each grants itself and everything below it,
the placeholders and `~` stand for the directories of `files.paths()`, and paths are compared after
following links and `..`. A refused path rejects with `NotAllowed`, for `serve` too. Socket paths are
checked on their own, apart from the file limit.

## Under the hood

The APIs share one path:

1. The app calls `host.io(op, call, args, bytes)`. `runtime/io/channel.ts` turns this
   into a promise, or into a stream of events for sockets and watches.
2. The host's `Io` (`host/src/io/`) starts the work on a thread: `fetch.rs`,
   `websocket.rs`, `files.rs`, `process.rs` or `socket.rs`. Dialogs go to the window instead, because they need
   GPUI.
3. The thread sends its result down a channel. The window wakes as soon as a result
   arrives, hands it to the app (`__io`), and draws what changed.

Bytes travel as `ArrayBuffer`s, and everything else as JSON.

Calls into your own Rust code (`invoke`, `listen`) take the same path: see
[Calling Rust](rust.md#under-the-hood).
