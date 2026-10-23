import type { BusService } from '../../lib/bus.js'

// The lock's one source of truth. Everything that wants the screen locked —
// the power menu's `loginctl lock-session`, a compositor keybind emitting
// lock:lock, hypridle — ends up in set(); the view that holds the compositor
// lock follows the retained topic. The session's LockedHint is kept in step so
// `loginctl` reports it.

export type LockedHintWriter = (locked: boolean) => void

export class LockState {
  private readonly bus: BusService
  private readonly writeHint: LockedHintWriter
  private locked = false
  private withdraw: () => void = () => {}

  constructor(bus: BusService, writeHint: LockedHintWriter) {
    this.bus = bus
    this.writeHint = writeHint
  }

  start(): () => void {
    this.publish()
    return () => {
      this.withdraw()
      this.withdraw = () => {}
    }
  }

  isLocked(): boolean {
    return this.locked
  }

  set(locked: boolean): void {
    if (locked === this.locked) {
      return
    }
    this.locked = locked
    this.publish()
    this.writeHint(locked)
  }

  private publish(): void {
    this.withdraw()
    this.withdraw = this.bus.retain('lock.state', { locked: this.locked })
  }
}
