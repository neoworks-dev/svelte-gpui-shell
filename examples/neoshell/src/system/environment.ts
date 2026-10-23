import { joinPath } from './fs.js'

// How this run differs from a normal session, read from the environment so a
// test can point the shell at a headless output and away from the user's own
// config, control socket and compositor.
//
//   NEOSHELL_CONFIG    config file path
//   NEOSHELL_SOCKET    control socket path
//   NEOSHELL_MONITORS  comma-separated output names; surfaces open only there
//   NEOSHELL_READONLY  1 keeps the shell from changing the system: a bus
//                      function that is not on the read-only list refuses
//   NEOSHELL_ALLOW     comma-separated bus functions a read-only run may
//                      still answer, for testing one against a scratch path
//   NEOSHELL_WINDOW    1 also opens a small plain window, which is how
//                      scripts/shot.sh finds an app that has only surfaces

export interface ShellEnvironment {
  configPath: string
  socketPath: string
  monitors: string[] | null
  readOnly: boolean
  allowedFunctions: string[]
  testWindow: boolean
}

export function environmentOf(
  variables: Record<string, string>,
  home: string,
  runtime: string,
): ShellEnvironment {
  return {
    configPath: textOf(variables.NEOSHELL_CONFIG, joinPath(home, '.config', 'neoshell', 'config.json')),
    socketPath: textOf(variables.NEOSHELL_SOCKET, joinPath(runtime, 'neoshell-host.sock')),
    monitors: listOfOrNull(variables.NEOSHELL_MONITORS),
    readOnly: variables.NEOSHELL_READONLY === '1',
    allowedFunctions: listOf(variables.NEOSHELL_ALLOW),
    testWindow: variables.NEOSHELL_WINDOW === '1',
  }
}

function textOf(value: string | undefined, fallback: string): string {
  if (value === undefined || value === '') {
    return fallback
  }
  return value
}

function listOf(value: string | undefined): string[] {
  const names = listOfOrNull(value)
  if (names === null) {
    return []
  }
  return names
}

function listOfOrNull(value: string | undefined): string[] | null {
  if (value === undefined || value === '') {
    return null
  }
  return value
    .split(',')
    .map((name) => name.trim())
    .filter((name) => name !== '')
}

export function isShellEnvironment(value: unknown): value is ShellEnvironment {
  return typeof value === 'object' && value !== null && 'configPath' in value && 'readOnly' in value
}

// Functions that only read, so a read-only run still answers them.
const READ_ONLY_FUNCTIONS = new Set(['apps:list', 'files:handlers', 'files:images', 'hypr:capture'])

let current: ShellEnvironment | null = null

export function useShellEnvironment(environment: ShellEnvironment): void {
  current = environment
}

export function mayAnswer(type: string): boolean {
  if (current === null || !current.readOnly) {
    return true
  }
  return READ_ONLY_FUNCTIONS.has(type) || current.allowedFunctions.includes(type)
}
