import { unixSockets } from '@neoworks/svelte-gpui'
import type { SocketConnection } from '@neoworks/svelte-gpui'

// Unix-socket clients and a line server. Two shapes are needed: a one-shot
// request that reads to EOF (Hyprland's request IPC) and an endless line stream
// that reconnects (Hyprland's event socket). The line server takes NDJSON from
// `neoshell emit` on a compositor keybind, and pushes bus topics to peers that
// subscribed.

// request writes one command and resolves with everything the peer sends back
// before closing. Hyprland answers exactly this way: one command per
// connection, reply, EOF.
export function request(socketPath: string, command: string, timeoutMs: number): Promise<string> {
  return unixSockets.request(socketPath, command, { timeout: timeoutMs })
}

interface StreamState {
  stopped: boolean
  connection: SocketConnection | null
  retryTimer: ReturnType<typeof setTimeout> | null
}

// streamLines follows a socket that stays open, reconnecting after every drop
// until the disposer runs: a compositor restarting must not kill the bridge.
export function streamLines(
  socketPath: string,
  onLine: (line: string) => void,
  reconnectDelayMs: number,
): () => void {
  const state: StreamState = { stopped: false, connection: null, retryTimer: null }
  openStream(socketPath, onLine, reconnectDelayMs, state)
  return () => {
    state.stopped = true
    if (state.retryTimer !== null) {
      clearTimeout(state.retryTimer)
    }
    state.connection?.close()
    state.connection = null
  }
}

function openStream(
  socketPath: string,
  onLine: (line: string) => void,
  reconnectDelayMs: number,
  state: StreamState,
): void {
  if (state.stopped) {
    return
  }
  const reconnect = (): void => scheduleReconnect(socketPath, onLine, reconnectDelayMs, state)
  state.connection = unixSockets.connect(socketPath, {
    onLine,
    onClose: reconnect,
    onError: reconnect,
  })
}

function scheduleReconnect(
  socketPath: string,
  onLine: (line: string) => void,
  reconnectDelayMs: number,
  state: StreamState,
): void {
  if (state.stopped || state.retryTimer !== null) {
    return
  }
  state.retryTimer = setTimeout(() => {
    state.retryTimer = null
    openStream(socketPath, onLine, reconnectDelayMs, state)
  }, reconnectDelayMs)
}

export interface LinePeer {
  readonly id: number
  write(data: string): void
}

export interface LineServerHandlers {
  onPeer(peer: LinePeer): void
  onLine(peer: LinePeer, line: string): void
  onPeerGone(peer: LinePeer): void
}

// serve accepts NDJSON peers on a unix socket and keeps each one so the
// handler can write to it at any time. The host removes a socket file nothing
// answers on, so a crashed run does not block the bind. A written line carries
// its own newline.
export function serve(socketPath: string, handlers: LineServerHandlers): () => void {
  const server = unixSockets.serve(
    socketPath,
    (line, peer) => {
      handlers.onLine(peer, line)
    },
    {
      framing: 'line',
      onPeer: handlers.onPeer,
      onPeerGone: handlers.onPeerGone,
      onError: (error) => console.error(`socket: ${socketPath}:`, error.message),
    },
  )
  server.ready.catch(() => {})
  return () => server.close()
}
