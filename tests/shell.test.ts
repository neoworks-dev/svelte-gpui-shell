import { beforeAll, describe, expect, test } from "bun:test";
import protocol from "../protocol.json";

const code = (name: string): number => protocol.operations.indexOf(name);
const CREATE_ELEMENT = code("createElement");
const SET_ATTRIBUTE = code("setAttribute");
const SET_TEXT = code("setText");
const REMOVE = code("remove");

type Operation = unknown[];
interface Run {
  mounted: Operation[];
  unmarked: Operation[];
  marked: Operation[];
  locking: Operation[];
  locked: Operation[];
  unlocked: Operation[];
}

let run: Run;
let marker: number;

beforeAll(async () => {
  const driver = new URL("./fixtures/drive-shell.ts", import.meta.url).pathname;
  const child = Bun.spawn(["bun", driver], { stdout: "pipe", stderr: "pipe" });
  const [output, errors] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text()]);
  expect(await child.exited, errors).toBe(0);
  run = JSON.parse(output.slice(output.lastIndexOf("@@") + 2));
  marker = run.mounted.find((op) => op[0] === CREATE_ELEMENT && op[2] === "g-view")![1] as number;
});

describe("input regions", () => {
  test("a View with input marks its element for the host", () => {
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, marker, "input", ""]);
  });

  test("a Popover with input marks its overlay", () => {
    const overlay = run.mounted.find((op) => op[0] === CREATE_ELEMENT && op[2] === "g-overlay")![1];
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, overlay, "input", ""]);
  });

  test("the mark goes when the prop does, and comes back with it", () => {
    expect(run.unmarked).toContainEqual([SET_ATTRIBUTE, marker, "input", null]);
    expect(run.marked).toContainEqual([SET_ATTRIBUTE, marker, "input", ""]);
  });
});

describe("session lock", () => {
  const shows = (operations: Operation[], text: string) =>
    operations.some((op) => op[0] === SET_TEXT && String(op[2]) === text);

  test("a Window with lock asks for a lock surface on its monitor", () => {
    const windows = run.locking.filter((op) => op[0] === CREATE_ELEMENT && op[2] === "g-window");
    expect(windows).toHaveLength(1);
    const lockWindow = windows[0][1];
    expect(run.locking).toContainEqual([SET_ATTRIBUTE, lockWindow, "lock", ""]);
    expect(run.locking).toContainEqual([SET_ATTRIBUTE, lockWindow, "monitor", "DP-2"]);
  });

  test("sessionLock.state follows what the host reports", () => {
    expect(shows(run.locked, "lock is locked")).toBe(true);
    expect(shows(run.unlocked, "lock is unlocked")).toBe(true);
  });

  test("unmounting the lock windows removes them from the tree", () => {
    expect(run.unlocked.some((op) => op[0] === REMOVE)).toBe(true);
  });
});

describe("video", () => {
  const video = () => run.mounted.find((op) => op[0] === CREATE_ELEMENT && op[2] === "g-video")![1];

  test("a Video becomes a g-video carrying its source and settings", () => {
    const id = video();
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "src", "/clips/loop.mp4"]);
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "fit", "cover"]);
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "rate", "2"]);
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "max-fps", "30"]);
  });

  test("the defaults are sent as nothing, and only the opposite as an attribute", () => {
    const id = video();
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "loop", "false"]);
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "muted", "false"]);
    expect(run.mounted).toContainEqual([SET_ATTRIBUTE, id, "paused", ""]);
  });
});
