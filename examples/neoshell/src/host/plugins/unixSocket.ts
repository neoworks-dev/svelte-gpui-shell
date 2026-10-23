import type { Plugin } from '@neoworks/extension-system'
import { requireService } from '../services.js'
import type { Bus } from '../bus.js'
import { WireConnection } from '../wire.js'
import { serve } from '../../system/socket.js'
import type { LinePeer } from '../../system/socket.js'

export interface UnixSocketConfig {
  socketPath: string
}

// unixSocketPlugin exposes the bus to local processes as NDJSON over a unix
// socket, for the outside world: `neoshell emit` bound to a compositor keybind,
// and any daemon an extension chooses to spawn.
export const unixSocketPlugin: Plugin.Object<UnixSocketConfig> = {
  name: 'unix-socket',
  inject: ['bus'],
  apply(context, config) {
    const bus = requireService<Bus>(context, 'bus')
    context.effect(() => listenUnix(bus, config.socketPath))
  },
}

// One WireConnection per peer: its subscriptions are pushed to it as lines, and
// its retained values and subscriptions go when it leaves.
export function listenUnix(bus: Bus, socketPath: string): () => void {
  const connections = new Map<number, WireConnection>()
  const stop = serve(socketPath, {
    onPeer(peer) {
      connections.set(peer.id, new WireConnection(bus, (line) => writeToPeer(peer, connections, line)))
    },
    onLine(peer, line) {
      connections.get(peer.id)?.feed(line + '\n')
    },
    onPeerGone(peer) {
      connections.get(peer.id)?.close()
      connections.delete(peer.id)
    },
  })
  return () => {
    stop()
    for (const connection of connections.values()) {
      connection.close()
    }
    connections.clear()
  }
}

// A write throws once the client has left; the connection is dropped so its
// subscriptions stop firing.
function writeToPeer(peer: LinePeer, connections: Map<number, WireConnection>, line: string): void {
  try {
    peer.write(line)
  } catch {
    connections.get(peer.id)?.close()
    connections.delete(peer.id)
  }
}
