// The library must be imported before anything that pulls in Svelte's client
// runtime: it installs the DOM globals that runtime reads at module scope.
import { start } from '@neoworks/svelte-gpui'
import { Bus } from './host/bus.js'
import { ViewRegistry } from './host/plugins/views.js'
import { publishSmokeData } from './smokeData.js'
import Smoke from './Smoke.svelte'

// Builds every view in the shell once, in a plain window. Nothing here touches
// the layer shell, a service or the user's session: the state the views read
// is published onto a bus of its own, and every command is answered with ok.
//
//   SHELL_BUNDLE=examples/neoshell/dist/smoke.js examples/neoshell/shot.sh OUT.png

const bus = new Bus()
const registry = new ViewRegistry()

publishSmokeData(bus, registry)

start(Smoke, { bus, registry }, { window: { title: 'neoshell smoke', width: 1500, height: 900 } })
