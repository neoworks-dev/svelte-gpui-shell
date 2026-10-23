// The monitors the compositor offers. The host reports them through `receive` when they
// change, so a component that reads `monitors.list` renders again when one is plugged in or
// unplugged, and a `Window` per monitor follows.

/** One monitor, in logical pixels. */
export interface Monitor {
  /** The output's connector, such as `DP-2`. A `Window`'s `monitor` names it. */
  readonly name: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

let list = $state<readonly Monitor[]>([]);

/** Takes the monitors the host reports. */
export function receive(next: Monitor[]): void {
  list = next;
}

export const monitors = {
  /** The monitors that are connected, as the host last reported them. Empty until it has. */
  get list(): readonly Monitor[] {
    return list;
  },
};
