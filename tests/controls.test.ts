import { describe, expect, test } from "bun:test";
import { levelColor, nudge, share, snap, valueAt } from "../runtime/ui/controls";

const percent = { min: 0, max: 100, step: 1 };

describe("snap", () => {
  test("rounds to the nearest step and stays inside the range", () => {
    expect(snap(42.4, percent)).toBe(42);
    expect(snap(-5, percent)).toBe(0);
    expect(snap(180, percent)).toBe(100);
    expect(snap(12, { min: 0, max: 100, step: 5 })).toBe(10);
    expect(snap(13, { min: 0, max: 100, step: 5 })).toBe(15);
  });

  test("counts steps from min, not from zero", () => {
    expect(snap(7, { min: 3, max: 20, step: 5 })).toBe(8);
  });

  test("does not leave floating point dust", () => {
    expect(snap(0.30000000000000004, { min: 0, max: 1, step: 0.1 })).toBe(0.3);
    expect(snap(0.7, { min: 0, max: 1, step: 0.05 })).toBe(0.7);
  });

  test("a step of zero allows any value, and a collapsed range its only one", () => {
    expect(snap(33.333, { min: 0, max: 100, step: 0 })).toBe(33.333);
    expect(snap(5, { min: 10, max: 10, step: 1 })).toBe(10);
  });

  test("never goes past max when the range is not a whole number of steps", () => {
    expect(snap(10, { min: 0, max: 10, step: 3 })).toBe(9);
    expect(snap(100, { min: 0, max: 10, step: 4 })).toBeLessThanOrEqual(10);
  });
});

describe("share", () => {
  test("is where the value lies in the range", () => {
    expect(share(25, percent)).toBe(0.25);
    expect(share(-3, percent)).toBe(0);
    expect(share(500, percent)).toBe(1);
    expect(share(5, { min: 10, max: 10, step: 1 })).toBe(0);
  });
});

describe("valueAt", () => {
  test("maps the pointer over the knob's travel, half a knob in from each end", () => {
    expect(valueAt(8, 216, 16, percent)).toBe(0);
    expect(valueAt(108, 216, 16, percent)).toBe(50);
    expect(valueAt(208, 216, 16, percent)).toBe(100);
  });

  test("clamps a pointer that left the track", () => {
    expect(valueAt(-300, 216, 16, percent)).toBe(0);
    expect(valueAt(900, 216, 16, percent)).toBe(100);
  });

  test("snaps to the step", () => {
    expect(valueAt(108, 216, 16, { min: 0, max: 10, step: 2 })).toBe(6);
  });

  test("a track not yet measured gives the start", () => {
    expect(valueAt(30, 0, 16, percent)).toBe(0);
    expect(valueAt(30, 16, 16, { min: 5, max: 9, step: 1 })).toBe(5);
  });
});

describe("nudge", () => {
  test("arrows move by a step and clamp", () => {
    expect(nudge(50, "ArrowRight", percent)).toBe(51);
    expect(nudge(50, "ArrowUp", percent)).toBe(51);
    expect(nudge(50, "ArrowLeft", percent)).toBe(49);
    expect(nudge(0, "ArrowDown", percent)).toBe(0);
    expect(nudge(100, "ArrowRight", percent)).toBe(100);
  });

  test("page keys move ten steps, home and end the extremes", () => {
    expect(nudge(50, "PageUp", percent)).toBe(60);
    expect(nudge(50, "PageDown", { min: 0, max: 100, step: 2 })).toBe(30);
    expect(nudge(50, "Home", percent)).toBe(0);
    expect(nudge(50, "End", percent)).toBe(100);
  });

  test("a slider with no step moves by a hundredth of its range", () => {
    expect(nudge(0.5, "ArrowRight", { min: 0, max: 1, step: 0 })).toBeCloseTo(0.51);
  });

  test("other keys are left alone", () => {
    expect(nudge(50, "a", percent)).toBeUndefined();
    expect(nudge(50, "Enter", percent)).toBeUndefined();
  });
});

describe("levelColor", () => {
  const levels = [
    { upTo: 40, color: "amber" },
    { upTo: 15, color: "red" },
    { upTo: 100, color: "green" },
  ];

  test("is the first level the value does not exceed, whatever order they come in", () => {
    expect(levelColor(10, levels, "plain")).toBe("red");
    expect(levelColor(15, levels, "plain")).toBe("red");
    expect(levelColor(16, levels, "plain")).toBe("amber");
    expect(levelColor(80, levels, "plain")).toBe("green");
  });

  test("past every level it keeps the last colour", () => {
    expect(levelColor(250, levels, "plain")).toBe("green");
  });

  test("without levels it is the fallback", () => {
    expect(levelColor(50, undefined, "plain")).toBe("plain");
    expect(levelColor(50, [], "plain")).toBe("plain");
  });
});
