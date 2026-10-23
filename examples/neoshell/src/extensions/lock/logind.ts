// The parts of logind's wire format the lock backend reads, as pure functions.
//
// `busctl call` prints a reply as a type signature followed by the values, for
// example `o "/org/freedesktop/login1/session/_33"` or `v s "3"`. `gdbus
// monitor` prints one signal per line, for example
// `/org/freedesktop/login1/session/_33: org.freedesktop.login1.Session.Lock ()`.

export type SessionSignal = 'lock' | 'unlock'

const SESSION_LOCK_SIGNAL = 'org.freedesktop.login1.Session.Lock'
const SESSION_UNLOCK_SIGNAL = 'org.freedesktop.login1.Session.Unlock'

export function quotedValueOf(reply: string): string | null {
  const match = reply.match(/"((?:[^"\\]|\\.)*)"/)
  if (match === null) {
    return null
  }
  return match[1]
}

export function sessionSignalOf(line: string): SessionSignal | null {
  const trimmed = line.trim()
  if (trimmed.includes(`${SESSION_LOCK_SIGNAL} `)) {
    return 'lock'
  }
  if (trimmed.includes(`${SESSION_UNLOCK_SIGNAL} `)) {
    return 'unlock'
  }
  return null
}
