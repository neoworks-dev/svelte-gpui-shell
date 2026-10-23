import { describe, expect, test } from "bun:test";
import { inflateSync } from "node:zlib";
import { parseCaptureFrame } from "../examples/neoshell/src/lib/captureFrame";
import { cubicBezier } from "../examples/neoshell/src/lib/easing";
import { sampleKeyframes } from "../examples/neoshell/src/lib/keyframes";
import { adler32, crc32, encodePng } from "../examples/neoshell/src/lib/png";
import { dockEntries, entryForWindow, magnificationOf, openWindowsOf } from "../examples/neoshell/src/extensions/neoshell/dockModel";
import type { DockApp, OpenWindow } from "../examples/neoshell/src/extensions/neoshell/dockModel";
import { environmentOf } from "../examples/neoshell/src/system/environment";
import { userDirectoryOf } from "../examples/neoshell/src/system/userDirs";
import { anchorOf, exclusiveZoneOf, keyboardOf, sizeOf } from "../examples/neoshell/src/tree";

describe("png", () => {
  test("checksums match the known values", () => {
    const text = new TextEncoder().encode("123456789");
    expect(crc32(text)).toBe(0xcbf43926);
    expect(adler32(text)).toBe(0x091e01de);
  });

  test("a frame survives the round trip through a real inflater, stride padding dropped", () => {
    const width = 3;
    const height = 2;
    const stride = 16;
    const pixels = new Uint8Array(stride * height).map((_, index) => index % 251);
    const png = encodePng(width, height, stride, pixels);
    expect(Array.from(png.subarray(0, 8))).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const idatLength = new DataView(png.buffer).getUint32(33);
    const idat = png.subarray(41, 41 + idatLength);
    const scanlines = inflateSync(idat);
    expect(scanlines.length).toBe(height * (width * 4 + 1));
    expect(scanlines[0]).toBe(0);
    expect(Array.from(scanlines.subarray(1, 13))).toEqual(Array.from(pixels.subarray(0, 12)));
    expect(Array.from(scanlines.subarray(14, 26))).toEqual(Array.from(pixels.subarray(16, 28)));
  });

  test("more than one stored block is still one stream", () => {
    const width = 200;
    const height = 200;
    const pixels = new Uint8Array(width * height * 4).map((_, index) => (index * 7) % 256);
    const png = encodePng(width, height, width * 4, pixels);
    const idatLength = new DataView(png.buffer).getUint32(33);
    const scanlines = inflateSync(png.subarray(41, 41 + idatLength));
    expect(scanlines.length).toBe(height * (width * 4 + 1));
  });
});

describe("capture frame", () => {
  test("reads the header and keeps the rows", () => {
    const raw = new Uint8Array(12 + 2 * 8);
    const view = new DataView(raw.buffer);
    view.setUint32(0, 2, true);
    view.setUint32(4, 2, true);
    view.setUint32(8, 8, true);
    const frame = parseCaptureFrame(raw);
    expect([frame.width, frame.height, frame.stride, frame.pixels.length]).toEqual([2, 2, 8, 16]);
  });

  test("refuses a truncated frame", () => {
    const raw = new Uint8Array(12 + 3);
    new DataView(raw.buffer).setUint32(0, 2, true);
    new DataView(raw.buffer).setUint32(4, 2, true);
    new DataView(raw.buffer).setUint32(8, 8, true);
    expect(() => parseCaptureFrame(raw)).toThrow("truncated");
  });
});

describe("easing and keyframes", () => {
  test("cubic-bezier keeps its ends and a linear curve is the identity", () => {
    const ease = cubicBezier(0.32, 1.24, 0.5, 1);
    expect(ease(0)).toBe(0);
    expect(ease(1)).toBe(1);
    const linear = cubicBezier(0, 0, 1, 1);
    expect(linear(0.37)).toBeCloseTo(0.37, 4);
  });

  test("a control point above one overshoots", () => {
    const ease = cubicBezier(0.32, 1.24, 0.5, 1);
    expect(Math.max(...[0.3, 0.4, 0.5, 0.6, 0.7].map(ease))).toBeGreaterThan(1);
  });

  test("keyframes interpolate between the stops around the progress", () => {
    const frames = [
      { at: 0, values: { lift: 0 } },
      { at: 0.5, values: { lift: 40 } },
      { at: 1, values: { lift: 0 } },
    ];
    expect(sampleKeyframes(frames, 0.25).lift).toBe(20);
    expect(sampleKeyframes(frames, 0.5).lift).toBe(40);
    expect(sampleKeyframes(frames, 0.75).lift).toBe(20);
    expect(sampleKeyframes(frames, 2).lift).toBe(0);
  });
});

describe("dock model", () => {
  const catalog: DockApp[] = [
    { id: "alacritty", name: "Alacritty", exec: "alacritty", icon: "Alacritty", wmClass: "Alacritty" },
    { id: "firefox", name: "Firefox", exec: "firefox", icon: "firefox", wmClass: "firefox" },
  ];

  function windowOf(wmClass: string, command = ""): OpenWindow {
    return { address: "0x1", wmClass, title: "", workspace: "1", pid: 1, command };
  }

  test("a window resolves by class, case-insensitively", () => {
    expect(entryForWindow(catalog, windowOf("FIREFOX")).id).toBe("firefox");
  });

  test("a window with its own class borrows the icon of the entry its process belongs to", () => {
    const entry = entryForWindow(catalog, windowOf("my-terminal", "alacritty"));
    expect(entry.id).toBe("my-terminal");
    expect(entry.icon).toBe("Alacritty");
  });

  test("a steam game gets steam's icon and anything else the generic one", () => {
    expect(entryForWindow(catalog, windowOf("steam_app_42")).icon).toBe("steam");
    expect(entryForWindow(catalog, windowOf("unknown")).icon).toBe("application-x-executable");
  });

  test("running apps follow the pinned ones and are not listed twice", () => {
    const entries = dockEntries(catalog, [catalog[0]], [windowOf("Alacritty"), windowOf("firefox")]);
    expect(entries.map((app) => app.id)).toEqual(["alacritty", "firefox"]);
  });

  test("windows without a class or an address are dropped", () => {
    expect(openWindowsOf([{ class: "", address: "0x1" }, { class: "a" }, { class: "a", address: "0x2", pid: 7 }])).toHaveLength(1);
  });

  test("magnification peaks under the pointer and fades over two neighbours", () => {
    expect(magnificationOf(2, 2)).toEqual({ scale: 1.45, shift: 0 });
    expect(magnificationOf(3, 2).shift).toBeGreaterThan(0);
    expect(magnificationOf(1, 2).shift).toBeLessThan(0);
    expect(magnificationOf(5, 2)).toEqual({ scale: 1, shift: 0 });
    expect(magnificationOf(0, -1)).toEqual({ scale: 1, shift: 0 });
  });
});

describe("environment", () => {
  test("defaults point at the user's config and control socket", () => {
    const environment = environmentOf({}, "/home/u", "/run/user/1");
    expect(environment.configPath).toBe("/home/u/.config/neoshell/config.json");
    expect(environment.socketPath).toBe("/run/user/1/neoshell-host.sock");
    expect(environment.monitors).toBeNull();
    expect(environment.readOnly).toBe(false);
  });

  test("overrides narrow the shell to a test setup", () => {
    const environment = environmentOf(
      { NEOSHELL_CONFIG: "/tmp/c.json", NEOSHELL_MONITORS: "a, b", NEOSHELL_READONLY: "1", NEOSHELL_ALLOW: "files:trash" },
      "/home/u",
      "/run/user/1",
    );
    expect(environment.configPath).toBe("/tmp/c.json");
    expect(environment.monitors).toEqual(["a", "b"]);
    expect(environment.readOnly).toBe(true);
    expect(environment.allowedFunctions).toEqual(["files:trash"]);
  });

  test("the desktop directory comes from user-dirs.dirs", () => {
    const text = '# comment\nXDG_DESKTOP_DIR="$HOME/Schreibtisch"\nXDG_MUSIC_DIR="$HOME/Music"\n';
    expect(userDirectoryOf(text, "XDG_DESKTOP_DIR", "/home/u")).toBe("/home/u/Schreibtisch");
    expect(userDirectoryOf(text, "XDG_VIDEOS_DIR", "/home/u")).toBeNull();
    expect(userDirectoryOf(null, "XDG_DESKTOP_DIR", "/home/u")).toBeNull();
  });
});

describe("view tree arguments", () => {
  test("layer surface fields read from a node's args", () => {
    expect(anchorOf({ anchors: ["top", "sideways", "left"] })).toEqual(["top", "left"]);
    expect(anchorOf({})).toEqual(["top", "bottom", "left", "right"]);
    expect(keyboardOf({ keyboard: "ondemand" })).toBe("on-demand");
    expect(keyboardOf({ keyboard: "bogus" })).toBe("none");
    expect(exclusiveZoneOf({})).toBe(-1);
    expect(exclusiveZoneOf({ exclusiveSize: 30 })).toBe(30);
    expect(sizeOf({ height: 30 }, "height")).toBe(30);
    expect(sizeOf({ height: -1 }, "height")).toBe(0);
  });
});
