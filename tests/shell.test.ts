import { beforeAll, describe, expect, test } from "bun:test";
import protocol from "../protocol.json";

const code = (name: string): number => protocol.operations.indexOf(name);
const CREATE_ELEMENT = code("createElement");
const SET_ATTRIBUTE = code("setAttribute");

type Operation = unknown[];
interface Run {
  mounted: Operation[];
  unmarked: Operation[];
  marked: Operation[];
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

  test("the mark goes when the prop does, and comes back with it", () => {
    expect(run.unmarked).toContainEqual([SET_ATTRIBUTE, marker, "input", null]);
    expect(run.marked).toContainEqual([SET_ATTRIBUTE, marker, "input", ""]);
  });
});
