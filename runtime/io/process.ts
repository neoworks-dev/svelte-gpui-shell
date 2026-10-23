// Processes, through the host. A program is run without a shell, from its argument list.
// `run` answers once the program has exited; `spawn` returns a handle and reports the
// program's output line by line as it writes it. The host side is host/src/io/process.rs.
//
// Nothing is limited unless the app declares a limit with `start(App, props, { process: {
// allow } })`. Environment variables the program sees are the host's own, with `env` laid
// over them.

import { request, stream, toBuffer } from "./channel";
import { TextDecoder, TextEncoder } from "./text";

declare const console: { error(...args: unknown[]): void };

/** What a failed process call says went wrong, in `ProcessError.code`. */
export type ProcessErrorCode = "NotFound" | "PermissionDenied" | "NotAllowed" | "InvalidArgument" | "TimedOut" | "Other";

/** The error a failed process call rejects with. `message` names the program and the reason. */
export class ProcessError extends Error {
  readonly code: ProcessErrorCode;

  constructor(code: ProcessErrorCode, message: string) {
    super(message);
    this.name = "ProcessError";
    this.code = code;
  }
}

/** A signal by name, or by its number on this system. */
export type Signal =
  | "SIGHUP"
  | "SIGINT"
  | "SIGQUIT"
  | "SIGABRT"
  | "SIGKILL"
  | "SIGUSR1"
  | "SIGUSR2"
  | "SIGPIPE"
  | "SIGALRM"
  | "SIGTERM"
  | "SIGCONT"
  | "SIGSTOP"
  | number;

/** What both kinds of run take. */
export interface ProcessOptions {
  /** The directory to run in. Defaults to the host's. */
  cwd?: string;
  /** Variables to set for the program, over the host's. `null` removes one. */
  env?: Record<string, string | null>;
}

export interface RunOptions extends ProcessOptions {
  /** Sent to the program's standard input, which is closed after it. Without it, the program's input is closed at once. */
  stdin?: string | Uint8Array;
  /** Kills the program and rejects with `TimedOut` if it runs longer, in milliseconds. */
  timeout?: number;
}

/** How a program ended. */
export interface ProcessExit {
  /** The exit code; 128 plus the signal for a program a signal ended, as a shell reports it. */
  status: number;
  /** The signal that ended the program, if one did. */
  signal?: string;
}

export interface RunResult extends ProcessExit {
  /** Everything the program wrote to standard output, as UTF-8. */
  stdout: string;
  stderr: string;
}

export interface SpawnOptions extends ProcessOptions {
  /** Called for each line the program writes to standard output, without its line ending. */
  onStdout?(line: string): void;
  /** Called for each line the program writes to standard error. */
  onStderr?(line: string): void;
  /** Called once the program has exited and its output has been delivered. */
  onExit?(exit: ProcessExit): void;
  /** Called when the program cannot be started. Defaults to `console.error`. */
  onError?(error: ProcessError): void;
}

export interface ProcessHandle {
  /** Settles when the program has exited, or rejects if it could not be started. */
  readonly exited: Promise<ProcessExit>;
  /** Sends text or bytes to the program's standard input. Throws once input is closed or the program has exited. */
  write(data: string | Uint8Array): void;
  /** Closes standard input, which most programs take as the end of their work. */
  closeStdin(): void;
  /** Signals the program and the processes it started (`"SIGTERM"` by default). */
  kill(signal?: Signal): void;
}

/**
 * The limit an app puts on the programs it runs. Each entry is a program name, looked up in
 * `PATH`, or an absolute path. A program is allowed when it is the same file as an entry.
 * An interpreter that is allowed can run anything, so allow the programs, not a shell.
 */
export interface ProcessLimit {
  allow: string[];
}

/** Turns the host's failure, which carries a `code`, into a `ProcessError`. */
function failure(error: unknown): ProcessError {
  if (error instanceof ProcessError) {
    return error;
  }
  const code = (error as { code?: unknown } | null)?.code;
  const message = error instanceof Error ? error.message : String(error);
  return new ProcessError(typeof code === "string" ? (code as ProcessErrorCode) : "Other", message);
}

async function call(op: string, args: unknown, bytes?: ArrayBuffer) {
  try {
    return await request(op, args, bytes);
  } catch (error) {
    throw failure(error);
  }
}

/** The bytes of text or of a typed array, as the host takes them. */
export function inputBuffer(data: string | Uint8Array): ArrayBuffer {
  if (typeof data === "string") {
    return toBuffer(new TextEncoder().encode(data));
  }
  return toBuffer(data);
}

/** The arguments every process call sends. Throws for an `argv` that cannot be run. */
export function launchArguments(argv: readonly string[], options: ProcessOptions): Record<string, unknown> {
  if (!Array.isArray(argv) || argv.length === 0 || argv.some((part) => typeof part !== "string")) {
    throw new ProcessError("InvalidArgument", "argv has to be a program and its arguments, all strings");
  }
  if (argv[0] === "") {
    throw new ProcessError("InvalidArgument", "argv needs a program to run");
  }
  return { argv, cwd: options.cwd, env: options.env };
}

/** Sets the limit. The host takes it once, before the app mounts, and refuses a second one. */
export function limitProcesses(limit: ProcessLimit): void {
  call("proc.limit", { allow: limit.allow }).catch((error) => console.error(error.message));
}

export const processes = {
  /**
   * Runs a program to its end. A program that exits with a failure status still resolves,
   * as a 404 does for `fetch`; only a program that cannot be started, or that times out, rejects.
   */
  async run(argv: readonly string[], options: RunOptions = {}): Promise<RunResult> {
    const args = { ...launchArguments(argv, options), timeout: options.timeout };
    const input = options.stdin === undefined ? undefined : inputBuffer(options.stdin);
    const reply = await call("proc.run", args, input);
    const output = new Uint8Array(reply.bytes ?? new ArrayBuffer(0));
    const { stdoutLength, ...exit } = reply.json;
    const decoder = new TextDecoder();
    return {
      ...exit,
      stdout: decoder.decode(output.subarray(0, stdoutLength)),
      stderr: decoder.decode(output.subarray(stdoutLength)),
    };
  },

  /** Starts a program and follows it. Output is delivered per line, as soon as the program writes it. */
  spawn(argv: readonly string[], options: SpawnOptions = {}): ProcessHandle {
    const args = launchArguments(argv, options);
    let settle: { resolve(exit: ProcessExit): void; reject(error: ProcessError): void };
    const exited = new Promise<ProcessExit>((resolve, reject) => {
      settle = { resolve, reject };
    });
    exited.catch(() => {});
    let over = false;
    let inputOpen = true;
    const process = stream("proc.spawn", args, {
      event: (reply) => {
        const handler = reply.json.type === "stderr" ? options.onStderr : options.onStdout;
        handler?.(reply.json.line);
      },
      close: (reply) => {
        over = true;
        const exit: ProcessExit = { status: reply.json.status };
        if (reply.json.signal !== undefined) {
          exit.signal = reply.json.signal;
        }
        settle.resolve(exit);
        options.onExit?.(exit);
      },
      reject: (error) => {
        over = true;
        const reason = failure(error);
        settle.reject(reason);
        (options.onError ?? ((problem) => console.error(problem.message)))(reason);
      },
    });
    const act = (op: string, extra: object, bytes?: ArrayBuffer): void => {
      call(op, { process, ...extra }, bytes).catch(() => {});
    };
    return {
      exited,
      write(data) {
        if (over || !inputOpen) {
          throw new ProcessError("Other", "the process takes no more input");
        }
        act("proc.write", {}, inputBuffer(data));
      },
      closeStdin() {
        if (over || !inputOpen) {
          return;
        }
        inputOpen = false;
        act("proc.closeStdin", {});
      },
      kill(signal = "SIGTERM") {
        if (!over) {
          act("proc.kill", { signal });
        }
      },
    };
  },

  /** The environment variables of the host, which programs inherit. */
  async env(): Promise<Record<string, string>> {
    return (await call("proc.env", {})).json.env;
  },
};
