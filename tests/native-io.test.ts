import { beforeEach, describe, expect, test } from "bun:test";
import { receive } from "../runtime/io/channel";
import { inputBuffer, launchArguments, ProcessError, processes } from "../runtime/io/process";
import { SocketError, unixSockets } from "../runtime/io/socket";

interface Sent {
  op: string;
  call: number;
  args: any;
  bytes?: ArrayBuffer;
}

let sent: Sent[];

/** What `host.io` would have received, with the host answering nothing on its own. */
beforeEach(() => {
  sent = [];
  (globalThis as any).host = {
    io(op: string, call: number, args: string, bytes?: ArrayBuffer) {
      sent.push({ op, call, args: JSON.parse(args), bytes });
    },
  };
});

function answer(call: number, settle: string, json: unknown = {}, bytes?: Uint8Array): void {
  const buffer = bytes === undefined ? undefined : (bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer);
  receive(call, settle, JSON.stringify(json), buffer);
}

const text = (buffer?: ArrayBuffer): string => new TextDecoder().decode(buffer);
/** Lets pending promise callbacks run; other test files replace the timers, so none is used. */
async function flush(): Promise<void> {
  for (let turn = 0; turn < 20; turn++) {
    await Promise.resolve();
  }
}

describe("launchArguments", () => {
  test("passes argv, cwd and env on", () => {
    expect(launchArguments(["ls", "-l"], { cwd: "/tmp", env: { A: "1", B: null } })).toEqual({
      argv: ["ls", "-l"],
      cwd: "/tmp",
      env: { A: "1", B: null },
    });
  });

  test("refuses an argv that cannot be run", () => {
    expect(() => launchArguments([], {})).toThrow(ProcessError);
    expect(() => launchArguments([""], {})).toThrow("needs a program");
    expect(() => launchArguments(["ls", 3 as unknown as string], {})).toThrow("all strings");
  });
});

test("inputBuffer encodes text as UTF-8 and copies exactly the viewed bytes", () => {
  expect(new Uint8Array(inputBuffer("é\0"))).toEqual(new Uint8Array([0xc3, 0xa9, 0]));
  const whole = new Uint8Array([1, 2, 3, 4]);
  expect(new Uint8Array(inputBuffer(whole.subarray(1, 3)))).toEqual(new Uint8Array([2, 3]));
});

describe("processes.run", () => {
  test("sends stdin as bytes and splits the reply into stdout and stderr", async () => {
    const result = processes.run(["cat"], { stdin: "pw\0", timeout: 500 });
    const [call] = sent;
    expect(call.op).toBe("proc.run");
    expect(call.args).toEqual({ argv: ["cat"], timeout: 500 });
    expect(new Uint8Array(call.bytes!)).toEqual(new Uint8Array([0x70, 0x77, 0]));
    answer(call.call, "resolve", { status: 1, stdoutLength: 3 }, new TextEncoder().encode("outerr"));
    expect(await result).toEqual({ status: 1, stdout: "out", stderr: "err" });
  });

  test("sends no bytes without stdin, and rejects with the host's code", async () => {
    const result = processes.run(["nope"]);
    expect(sent[0].bytes).toBeUndefined();
    answer(sent[0].call, "reject", { message: "cannot run nope: not found", code: "NotFound" });
    const error = await result.catch((reason) => reason);
    expect(error).toBeInstanceOf(ProcessError);
    expect(error.code).toBe("NotFound");
  });
});

describe("processes.spawn", () => {
  test("routes lines to their handlers and resolves exited", async () => {
    const lines: string[] = [];
    const exits: unknown[] = [];
    const handle = processes.spawn(["pactl", "subscribe"], {
      onStdout: (line) => lines.push(`out:${line}`),
      onStderr: (line) => lines.push(`err:${line}`),
      onExit: (exit) => exits.push(exit),
    });
    const { call } = sent[0];
    answer(call, "event", { type: "stdout", line: "a" });
    answer(call, "event", { type: "stderr", line: "b" });
    answer(call, "close", { status: 143, signal: "SIGTERM" });
    expect(lines).toEqual(["out:a", "err:b"]);
    expect(exits).toEqual([{ status: 143, signal: "SIGTERM" }]);
    expect(await handle.exited).toEqual({ status: 143, signal: "SIGTERM" });
  });

  test("writes, closes input once, kills, and refuses to write after the exit", async () => {
    const handle = processes.spawn(["cat"]);
    const { call } = sent[0];
    handle.write("hi");
    handle.closeStdin();
    handle.closeStdin();
    handle.kill("SIGKILL");
    expect(sent.slice(1).map((request) => [request.op, request.args])).toEqual([
      ["proc.write", { process: call }],
      ["proc.closeStdin", { process: call }],
      ["proc.kill", { process: call, signal: "SIGKILL" }],
    ]);
    expect(() => handle.write("more")).toThrow("no more input");
    answer(call, "close", { status: 0 });
    await handle.exited;
    handle.kill();
    expect(sent.length).toBe(4);
  });

  test("a program that cannot start rejects exited and calls onError", async () => {
    const errors: ProcessError[] = [];
    const handle = processes.spawn(["nope"], { onError: (error) => errors.push(error) });
    answer(sent[0].call, "reject", { message: "cannot run nope", code: "NotFound" });
    expect(errors.map((error) => error.code)).toEqual(["NotFound"]);
    expect(await handle.exited.catch((reason) => reason)).toBe(errors[0]);
  });
});

describe("unixSockets", () => {
  test("request decodes the answer as text", async () => {
    const result = unixSockets.request("/run/hypr/.socket.sock", "j/monitors", { timeout: 1000 });
    expect(sent[0].op).toBe("sock.request");
    expect(sent[0].args).toEqual({ path: "/run/hypr/.socket.sock", timeout: 1000 });
    expect(text(sent[0].bytes)).toBe("j/monitors");
    answer(sent[0].call, "resolve", {}, new TextEncoder().encode("[]"));
    expect(await result).toBe("[]");
  });

  test("connect reports lines and close, and its writes name the socket", async () => {
    const lines: string[] = [];
    const connection = unixSockets.connect("/s", { onLine: (line) => lines.push(line) });
    const { call } = sent[0];
    connection.write("subscribe\n");
    expect(sent[1]).toMatchObject({ op: "sock.write", args: { socket: call } });
    answer(call, "event", { type: "open" });
    answer(call, "event", { type: "line", line: "workspace>>1" });
    answer(call, "close", {});
    await connection.closed;
    expect(lines).toEqual(["workspace>>1"]);
    expect(() => connection.write("x")).toThrow(SocketError);
  });

  test("a failed connect rejects closed with the code", async () => {
    const connection = unixSockets.connect("/missing", { onError: () => {} });
    answer(sent[0].call, "reject", { message: "cannot connect to /missing", code: "NotFound" });
    const error = await connection.closed.catch((reason) => reason);
    expect(error.code).toBe("NotFound");
  });

  test("serve answers each request with the handler's result, awaited", async () => {
    const server = unixSockets.serve("/s", async (request) => (request === "ping" ? "pong" : undefined));
    const { call } = sent[0];
    expect(sent[0].args).toEqual({ path: "/s", framing: "eof" });
    answer(call, "event", { type: "listening" });
    await server.ready;
    answer(call, "event", { type: "request", id: 7, text: "ping" });
    answer(call, "event", { type: "request", id: 8, text: "other" });
    await flush();
    const replies = sent.filter((request) => request.op === "sock.reply");
    expect(replies.map((reply) => [reply.args, text(reply.bytes)])).toEqual([
      [{ request: 7, failed: false }, "pong"],
      [{ request: 8, failed: false }, ""],
    ]);
    server.close();
    server.close();
    expect(sent.filter((request) => request.op === "sock.stop")).toHaveLength(1);
  });

  test("serve tells the host a handler failed, and reports it", async () => {
    const problems: string[] = [];
    unixSockets.serve("/s", () => { throw new Error("boom"); }, { framing: "line", onError: (error) => problems.push(error.message) });
    answer(sent[0].call, "event", { type: "request", id: 1, text: "x" });
    await flush();
    expect(problems).toEqual(["boom"]);
    expect(sent[1]).toMatchObject({ op: "sock.reply", args: { request: 1, failed: true } });
  });
});

describe("binary process output", () => {
  test("run with encoding bytes returns the bytes of standard output untouched", async () => {
    const result = processes.run(["capture"], { encoding: "bytes" });
    await flush();
    answer(sent[0].call, "resolve", { status: 0, stdoutLength: 3 }, new Uint8Array([0xff, 0x00, 0x80, 0x65]));
    const done = await result;
    expect(done.stdout).toEqual(new Uint8Array([0xff, 0x00, 0x80]));
    expect(done.stderr).toBe("e");
  });

  test("run without it still decodes text", async () => {
    const result = processes.run(["echo"]);
    await flush();
    answer(sent[0].call, "resolve", { status: 0, stdoutLength: 2 }, new TextEncoder().encode("hi"));
    expect((await result).stdout).toBe("hi");
  });

  test("spawn with onStdoutBytes asks the host for chunks and hands them over", async () => {
    const chunks: number[][] = [];
    processes.spawn(["capture"], { onStdoutBytes: (chunk) => chunks.push([...chunk]) });
    expect(sent[0].args.binaryStdout).toBe(true);
    answer(sent[0].call, "event", { type: "stdoutBytes" }, new Uint8Array([1, 2, 255]));
    expect(chunks).toEqual([[1, 2, 255]]);
  });

  test("spawn without it keeps lines", () => {
    processes.spawn(["echo"], { onStdout: () => {} });
    expect(sent[0].args.binaryStdout).toBe(false);
  });
});
