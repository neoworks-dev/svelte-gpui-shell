import type { Context, Plugin } from '@neoworks/extension-system'
import { requireService } from '../../lib/services.js'
import { registerFunction } from '../../lib/bus.js'
import type { BusService } from '../../lib/bus.js'
import { mayAnswer } from '../../system/environment.js'
import { run, watchLines } from '../../system/proc.js'
import { LockState } from './lockState.js'
import { quotedValueOf, sessionSignalOf } from './logind.js'

// lock: the session lock's state, and its ties to logind.
//
//   lock.state     {locked}                retained
//   lock:lock      {}         → {ok}
//   lock:unlock    {}         → {ok}
//
// logind's Lock and Unlock signals on this session are followed so that
// `loginctl lock-session` and hypridle work without knowing about the bus. The
// session is resolved through /session/auto, which also works when the shell
// runs outside a session cgroup. A read-only run does not write the session's
// LockedHint: it belongs to the user's real session.

const LOGIND = 'org.freedesktop.login1'
const MANAGER_PATH = '/org/freedesktop/login1'
const SESSION_INTERFACE = 'org.freedesktop.login1.Session'
const AUTO_SESSION_PATH = '/org/freedesktop/login1/session/auto'
const HINT_FUNCTION = 'logind:setLockedHint'

const lockExtension: Plugin.Object = {
  name: 'lock',
  inject: ['bus'],
  apply(context) {
    const bus = requireService<BusService>(context, 'bus')
    const session = new LogindSession()
    const state = new LockState(bus, (locked) => session.setLockedHint(locked))
    context.effect(() => state.start())
    watchLogind(context, session, state)
    registerFunction(context, bus, 'lock:lock', () => {
      state.set(true)
      return { ok: true }
    })
    registerFunction(context, bus, 'lock:unlock', () => {
      state.set(false)
      return { ok: true }
    })
  },
}

export default lockExtension

class LogindSession {
  private path: string | null = null

  setPath(path: string): void {
    this.path = path
  }

  getPath(): string | null {
    return this.path
  }

  setLockedHint(locked: boolean): void {
    if (this.path === null || !mayAnswer(HINT_FUNCTION)) {
      return
    }
    void run(['busctl', '--system', 'call', LOGIND, this.path, SESSION_INTERFACE, 'SetLockedHint', 'b', String(locked)])
  }
}

function watchLogind(context: Context, session: LogindSession, state: LockState): void {
  context.effect(() => {
    let stop = () => {}
    let cancelled = false
    void resolveSessionPath().then((path) => {
      if (cancelled || path === null) {
        return
      }
      session.setPath(path)
      stop = watchLines(['gdbus', 'monitor', '--system', '--dest', LOGIND, '--object-path', path], (line) =>
        applySignal(state, line),
      )
    })
    return () => {
      cancelled = true
      stop()
    }
  })
}

function applySignal(state: LockState, line: string): void {
  const signal = sessionSignalOf(line)
  if (signal === 'lock') {
    state.set(true)
    return
  }
  if (signal === 'unlock') {
    state.set(false)
  }
}

async function resolveSessionPath(): Promise<string | null> {
  const identity = await run([
    'busctl', '--system', 'call', LOGIND, AUTO_SESSION_PATH,
    'org.freedesktop.DBus.Properties', 'Get', 'ss', SESSION_INTERFACE, 'Id',
  ])
  const id = quotedValueOf(identity.stdout)
  if (!identity.ok || id === null) {
    console.warn('lock: no logind session for this process:', identity.stderr)
    return null
  }
  const lookup = await run([
    'busctl', '--system', 'call', LOGIND, MANAGER_PATH, `${LOGIND}.Manager`, 'GetSession', 's', id,
  ])
  return quotedValueOf(lookup.stdout)
}
