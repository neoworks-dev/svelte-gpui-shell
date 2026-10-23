// Where the session lock is. A `Window` with `lock` asks for a lock surface on its monitor, and
// the session stays locked while any of them is mounted. The host reports the state through
// `receive` as the compositor confirms or ends the lock.

/**
 * - `"unlocked"`: no lock window is mounted.
 * - `"locking"`: lock windows are mounted and the compositor has not confirmed the lock yet.
 * - `"locked"`: the compositor confirmed it. The session is locked, and the lock windows are what shows.
 * - `"finished"`: lock windows are mounted but none is left, because the compositor refused or ended
 *   the lock. It goes back to `"unlocked"` when the app unmounts them.
 */
export type SessionLockState = "unlocked" | "locking" | "locked" | "finished";

let state = $state<SessionLockState>("unlocked");

/** Takes the state the host reports. */
export function receive(next: SessionLockState): void {
  state = next;
}

export const sessionLock = {
  /** Where the lock is, as the host last reported it. */
  get state(): SessionLockState {
    return state;
  },
};
