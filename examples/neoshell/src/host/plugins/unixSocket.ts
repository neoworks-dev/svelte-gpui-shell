import type { Plugin } from '@neoworks/extension-system'
import { requireService } from '../services.js'
import type { Bus } from '../bus.js'
import { WireConnection } from '../wire.js'
import { serve } from '../../system/socket.js'

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

// One connection stands for every peer: the host answers a line with the
// handler's return value only, so a peer cannot subscribe to what the bus
// publishes. Publishing and retaining is what `neoshell emit` needs.
function listenUnix(bus: Bus, socketPath: string): () => void {
  const connection = new WireConnection(bus, () => {})
  const stop = serve(socketPath, (line) => connection.feed(line + '\n'))
  return () => {
    stop()
    connection.close()
  }
}
