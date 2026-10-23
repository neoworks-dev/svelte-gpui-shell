// Order matters: the DOM globals have to exist before Svelte's client runtime
// is evaluated, and this is the module every entry point imports first.
import { pump, wake } from "./dom/globals";

import { flushSync, mount } from "svelte";
import { SElement } from "./dom/nodes";
import { adoptRoot, flush, install } from "./host/bridge";
import { dispatch } from "./host/events";
import { receive } from "./io/channel";
import { limitFiles, type FilesLimit } from "./io/files";
import { limitProcesses, type ProcessLimit } from "./io/process";
import { limitSockets, type SocketLimit } from "./io/socket";
import { receive as receiveMonitors } from "./monitors.svelte";
import { receive as receiveSessionLock, type SessionLockState } from "./session-lock.svelte";
import { askToClose, declareWindow, receive as receiveWindow, type WindowOptions } from "./window.svelte";

export type { HostEvent } from "./host/events";
export { window } from "./window.svelte";
export { monitors } from "./monitors.svelte";
export type { Monitor } from "./monitors.svelte";
export { sessionLock } from "./session-lock.svelte";
export type { SessionLockState } from "./session-lock.svelte";
export type { CloseHandler, WindowOptions } from "./window.svelte";
export { element } from "./ui/element";
export type { ElementMethods, ElementProps } from "./ui/element";
export { files, FileError } from "./io/files";
export { processes, ProcessError } from "./io/process";
export type {
  ProcessErrorCode,
  ProcessExit,
  ProcessHandle,
  ProcessLimit,
  ProcessOptions,
  RunOptions,
  RunResult,
  Signal,
  SpawnOptions,
} from "./io/process";
export { unixSockets, SocketError } from "./io/socket";
export type {
  ConnectOptions,
  RequestOptions,
  ServeAnswer,
  ServeOptions,
  SocketConnection,
  SocketErrorCode,
  SocketLimit,
  SocketServer,
} from "./io/socket";
export { invoke, invokeBytes, listen } from "./io/invoke";
export type { CommandMap, EventMap, InvokeBytes, InvokeResult } from "./io/invoke";
export type {
  FileEntry,
  FileErrorCode,
  FileKind,
  FilePaths,
  FileStat,
  FilesLimit,
  PickFilesOptions,
  PickSavePathOptions,
  WatchEvent,
  WatchOptions,
} from "./io/files";

/** What `start` takes beyond the component and its props. */
export interface StartOptions {
  /** Limits the files the app may reach. Without it, everything is allowed. */
  files?: FilesLimit;
  /** Limits the programs the app may run. Without it, any program is allowed. */
  process?: ProcessLimit;
  /** Limits the unix socket paths the app may use. Without it, any path is allowed. */
  sockets?: SocketLimit;
  /**
   * How the window opens. The host's `--width` and `--height` flags override its size.
   * `false` opens no window of the app's own: it draws only in its `Window`s.
   */
  window?: WindowOptions | false;
}

/**
 * Mounts a component into the host's root, and exposes the entry points the host
 * calls back into: `__event` for input, `__io` for background calls finishing,
 * `__window` for the window's state, `__monitors` for the monitors, `__sessionLock` for the lock, `__closeRequested` for the close button, and `__pump` for the clock, which answers when it next has work (see `wake`).
 */
export function start(Component: unknown, props: Record<string, unknown> = {}, options: StartOptions = {}): void {
  // The limits go first, so nothing the app does while mounting can run without them.
  if (options.files !== undefined) {
    limitFiles(options.files);
  }
  if (options.process !== undefined) {
    limitProcesses(options.process);
  }
  if (options.sockets !== undefined) {
    limitSockets(options.sockets);
  }
  if (options.window === false) {
    declareWindow({ none: true });
  } else if (options.window !== undefined) {
    declareWindow(options.window);
  }
  install();
  const root = new SElement("#root");
  adoptRoot(root);

  const settle = (): void => {
    flushSync();
    flush();
  };

  mount(Component as any, { target: root as any, props });
  settle();

  const globals = globalThis as any;
  globals.__settle = settle;
  globals.__event = (id: number, type: string, detail: string): string => {
    const outcome = dispatch(id, type, JSON.parse(detail));
    settle();
    return JSON.stringify(outcome);
  };
  globals.__io = receive;
  globals.__window = (
    width: number,
    height: number,
    scale: number,
    focused: boolean,
    maximized: boolean,
    fullscreen: boolean,
  ): void => {
    globals.innerWidth = width;
    globals.innerHeight = height;
    if (receiveWindow(width, height, scale, focused, maximized, fullscreen)) {
      dispatch(0, "resize", {});
    }
    settle();
  };
  globals.__monitors = (list: string): void => {
    receiveMonitors(JSON.parse(list));
    settle();
  };
  globals.__sessionLock = (state: SessionLockState): void => {
    receiveSessionLock(state);
    settle();
  };
  globals.__closeRequested = askToClose;
  globals.__pump = (clock: number, frame: boolean): number => {
    if (pump(clock, frame)) {
      settle();
    }
    return wake();
  };
}

/**
 * Frames the window drew in the last second, and the average time building their
 * elements took. While something moves, frames follow the display's refresh rate;
 * GPUI only draws when something changed, so an idle window reports few.
 */
export function frameStats(): { fps: number; buildMs: number } {
  const [fps, buildMs] = (globalThis as any).host.frameStats() as [number, number];
  return { fps, buildMs };
}

/** The system clipboard, through the host. */
export const clipboard = {
  write(text: string): Promise<void> {
    return (globalThis as any).navigator.clipboard.writeText(text);
  },
  read(): Promise<string> {
    return (globalThis as any).navigator.clipboard.readText();
  },
};
