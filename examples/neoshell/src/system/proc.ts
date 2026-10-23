import { processes } from '@neoworks/svelte-gpui'
import type { ProcessHandle, RunResult } from '@neoworks/svelte-gpui'

// Subprocess glue for the extensions that wrap a CLI tool (nmcli,
// bluetoothctl, brightnessctl, playerctl). Commands are argv arrays, never
// shell strings, so an SSID or device name carrying quotes or spaces needs no
// escaping.

export interface CommandResult {
  ok: boolean
  stdout: string
  stderr: string
}

export async function run(command: string[]): Promise<CommandResult> {
  try {
    return commandResultOf(await processes.run(command))
  } catch (error) {
    return { ok: false, stdout: '', stderr: String(error) }
  }
}

// runWithInput is run with something written to the child's stdin: the way a
// helper takes a secret, so it never appears in an argv that `ps` can read.
// The input goes as bytes, so a NUL inside it (unix_chkpwd's password
// terminator) survives.
export async function runWithInput(command: string[], input: Uint8Array): Promise<CommandResult> {
  try {
    return commandResultOf(await processes.run(command, { stdin: input }))
  } catch (error) {
    return { ok: false, stdout: '', stderr: String(error) }
  }
}

// output returns stdout and swallows failure: for the many read-only probes
// where "the tool is missing" and "the tool found nothing" mean the same thing
// to the caller.
export async function output(command: string[]): Promise<string> {
  const result = await run(command)
  if (!result.ok) {
    return ''
  }
  return result.stdout
}

// watchLines streams a long-running command and calls onLine per line, then
// onEnd once the command exits: a tool that exits when its subject goes away
// (playerctl losing the last player) is how a caller learns to restart it. The
// returned disposer kills the child and does not fire onEnd, since the caller
// asked for the stop.
export function watchLines(
  command: string[],
  onLine: (line: string) => void,
  onEnd?: () => void,
): () => void {
  const state = { stopped: false }
  let handle: ProcessHandle
  try {
    handle = processes.spawn(command, {
      onStdout: onLine,
      onExit: () => endUnlessStopped(state, onEnd),
      onError: (error) => {
        console.error(`proc: cannot watch ${command[0]}:`, error.message)
        endUnlessStopped(state, onEnd)
      },
    })
  } catch (error) {
    console.error(`proc: cannot watch ${command[0]}:`, error)
    return () => {}
  }
  handle.exited.catch(() => {})
  return () => {
    state.stopped = true
    handle.kill()
  }
}

function endUnlessStopped(state: { stopped: boolean }, onEnd: (() => void) | undefined): void {
  if (state.stopped || onEnd === undefined) {
    return
  }
  state.stopped = true
  onEnd()
}

function commandResultOf(result: RunResult): CommandResult {
  return {
    ok: result.status === 0,
    stdout: result.stdout.trim(),
    stderr: result.stderr.trim(),
  }
}

// nmcli's terse mode separates fields with ":" and escapes literal colons as
// "\:", so a naive split shreds SSIDs and MAC addresses. splitTerse walks the
// line instead.
export function splitTerse(line: string): string[] {
  const fields: string[] = []
  let current = ''
  let escaped = false
  for (const character of line) {
    ;({ current, escaped } = consumeTerseCharacter(fields, current, escaped, character))
  }
  fields.push(current)
  return fields
}

function consumeTerseCharacter(
  fields: string[],
  current: string,
  escaped: boolean,
  character: string,
): { current: string; escaped: boolean } {
  if (escaped) {
    return { current: current + character, escaped: false }
  }
  if (character === '\\') {
    return { current, escaped: true }
  }
  if (character === ':') {
    fields.push(current)
    return { current: '', escaped: false }
  }
  return { current: current + character, escaped: false }
}

// launchDetached starts a program in its own session, so it outlives the shell:
// the host kills what the shell spawned when it quits, and setsid is what
// takes the child out of that reach. The child's output goes to /dev/null,
// because `run` would otherwise wait for the pipes it inherited to close.
// Resolves with an error message, or ''.
export async function launchDetached(command: string[]): Promise<string> {
  const script = 'exec "$@" >/dev/null 2>&1 </dev/null'
  const result = await run(['setsid', '--fork', 'sh', '-c', script, 'sh', ...command])
  if (result.ok) {
    return ''
  }
  if (result.stderr === '') {
    return `${command[0]} failed`
  }
  return result.stderr
}
