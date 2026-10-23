// Unix sockets, through the host: a one-shot request, a connection that stays open and
// reports lines, and a server. The host side is host/src/io/socket.rs.
//
// Nothing is limited unless the app declares a limit with `start(App, props, { sockets: {
// allow } })`.

import { request, stream } from "./channel";
import { inputBuffer } from "./process";
import { TextDecoder } from "./text";

declare const console: { error(...args: unknown[]): void };

/** What a failed socket call says went wrong, in `SocketError.code`. */
export type SocketErrorCode =
  | "NotFound"
  | "PermissionDenied"
  | "NotAllowed"
  | "ConnectionRefused"
  | "AddressInUse"
  | "TimedOut"
  | "InvalidArgument"
  | "Other";

/** The error a failed socket call rejects with. `message` names the path and the system's reason. */
export class SocketError extends Error {
  readonly code: SocketErrorCode;

  constructor(code: SocketErrorCode, message: string) {
    super(message);
    this.name = "SocketError";
    this.code = code;
  }
}

export interface RequestOptions {
  /** Gives up after this many milliseconds, rejecting with `TimedOut`. Defaults to 30000. */
  timeout?: number;
  /** Close the writing end after the request, so the server sees its end. Defaults to true. Hyprland needs it. */
  halfClose?: boolean;
}

export interface ConnectOptions {
  /** Called once the connection is made. */
  onOpen?(): void;
  /** Called for each line the peer sends, without its line ending. */
  onLine?(line: string): void;
  /** Called when the connection ends, whichever side ended it. */
  onClose?(): void;
  /** Called when the connection cannot be made. Defaults to `console.error`. */
  onError?(error: SocketError): void;
}

export interface SocketConnection {
  /** Settles when the connection has ended, or rejects if it could not be made. */
  readonly closed: Promise<void>;
  /** Queues text or bytes to send. They go out in order, once the connection is made. */
  write(data: string | Uint8Array): void;
  /** Ends the connection after what was queued has been sent. */
  close(): void;
}

/** What a server's handler returns: the answer, written back to the peer. Nothing writes nothing. */
export type ServeAnswer = string | Uint8Array | void;

export interface ServeOptions {
  /**
   * How requests are delimited. `"eof"` (the default): a request is everything the peer sends
   * until it closes its writing end, and the answer ends the connection, as Hyprland's does.
   * `"line"`: every line is a request and its answer is written as a line (a newline is added
   * if the answer lacks one), and the connection stays open until the peer leaves.
   */
  framing?: "eof" | "line";
  /** Called when the server cannot start or fails. Defaults to `console.error`. */
  onError?(error: SocketError): void;
}

export interface SocketServer {
  readonly path: string;
  /** Settles once the server listens, or rejects if it could not start. */
  readonly ready: Promise<void>;
  /** Stops listening, ends the connections and removes the socket file. */
  close(): void;
}

/**
 * The limit an app puts on the socket paths it uses. Each entry grants itself and everything
 * below it. `~` and the placeholders `$home`, `$config`, `$data`, `$cache`, `$temp`, `$runtime`,
 * `$bundle` and `$cwd` at the start of an entry stand for the directories of `files.paths()`.
 */
export interface SocketLimit {
  allow: string[];
}

function failure(error: unknown): SocketError {
  if (error instanceof SocketError) {
    return error;
  }
  const code = (error as { code?: unknown } | null)?.code;
  const message = error instanceof Error ? error.message : String(error);
  return new SocketError(typeof code === "string" ? (code as SocketErrorCode) : "Other", message);
}

async function call(op: string, args: unknown, bytes?: ArrayBuffer) {
  try {
    return await request(op, args, bytes);
  } catch (error) {
    throw failure(error);
  }
}

/** Sets the limit. The host takes it once, before the app mounts, and refuses a second one. */
export function limitSockets(limit: SocketLimit): void {
  call("sock.limit", { allow: limit.allow }).catch((error) => console.error(error.message));
}

async function exchange(path: string, data: string | Uint8Array, options: RequestOptions): Promise<Uint8Array> {
  const args = { path, timeout: options.timeout, halfClose: options.halfClose };
  const reply = await call("sock.request", args, inputBuffer(data));
  return new Uint8Array(reply.bytes ?? new ArrayBuffer(0));
}

export const unixSockets = {
  /** Connects, sends `data`, reads until the server closes, and resolves with what it sent as UTF-8. */
  async request(path: string, data: string | Uint8Array, options: RequestOptions = {}): Promise<string> {
    return new TextDecoder().decode(await exchange(path, data, options));
  },

  /** Like `request`, and resolves with the raw bytes. */
  requestBytes(path: string, data: string | Uint8Array, options: RequestOptions = {}): Promise<Uint8Array> {
    return exchange(path, data, options);
  },

  /** Connects and reports the lines the peer sends, as soon as they arrive. */
  connect(path: string, options: ConnectOptions = {}): SocketConnection {
    let settle: { resolve(): void; reject(error: SocketError): void };
    const closed = new Promise<void>((resolve, reject) => {
      settle = { resolve, reject };
    });
    closed.catch(() => {});
    let over = false;
    const socket = stream("sock.connect", { path }, {
      event: (reply) => {
        if (reply.json.type === "open") {
          options.onOpen?.();
        } else {
          options.onLine?.(reply.json.line);
        }
      },
      close: () => {
        over = true;
        settle.resolve();
        options.onClose?.();
      },
      reject: (error) => {
        over = true;
        const reason = failure(error);
        settle.reject(reason);
        (options.onError ?? ((problem) => console.error(problem.message)))(reason);
      },
    });
    const act = (op: string, bytes?: ArrayBuffer): void => {
      call(op, { socket }, bytes).catch(() => {});
    };
    return {
      closed,
      write(data) {
        if (over) {
          throw new SocketError("Other", `the connection to ${path} has ended`);
        }
        act("sock.write", inputBuffer(data));
      },
      close() {
        if (!over) {
          act("sock.close");
        }
      },
    };
  },

  /**
   * Listens on `path`, first removing a socket file that nothing answers on. The handler gets
   * each request's text and returns the answer, or a promise of it. A handler that throws or
   * rejects ends that connection without an answer.
   */
  serve(path: string, handler: (request: string) => ServeAnswer | Promise<ServeAnswer>, options: ServeOptions = {}): SocketServer {
    let settle: { resolve(): void; reject(error: SocketError): void };
    const ready = new Promise<void>((resolve, reject) => {
      settle = { resolve, reject };
    });
    ready.catch(() => {});
    const report = options.onError ?? ((problem: SocketError) => console.error(problem.message));
    let over = false;
    const answer = async (id: number, text: string): Promise<void> => {
      let reply: ArrayBuffer | undefined;
      try {
        const result = await handler(text);
        reply = inputBuffer(result === undefined ? "" : result);
      } catch (error) {
        report(failure(error));
      }
      call("sock.reply", { request: id, failed: reply === undefined }, reply).catch(() => {});
    };
    const server = stream("sock.serve", { path, framing: options.framing ?? "eof" }, {
      event: (reply) => {
        const event = reply.json;
        if (event.type === "listening") {
          settle.resolve();
        } else if (event.type === "request") {
          answer(event.id, event.text);
        } else {
          report(new SocketError("Other", event.message));
        }
      },
      close: () => {
        over = true;
        settle.resolve();
      },
      reject: (error) => {
        over = true;
        const reason = failure(error);
        settle.reject(reason);
        report(reason);
      },
    });
    return {
      path,
      ready,
      close() {
        if (!over) {
          over = true;
          call("sock.stop", { server }).catch(() => {});
        }
      },
    };
  },
};
