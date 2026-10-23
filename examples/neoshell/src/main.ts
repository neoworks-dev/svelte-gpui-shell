// The library must be imported before anything that pulls in Svelte's client
// runtime: it installs the DOM globals that runtime reads at module scope.
import { start } from '@neoworks/svelte-gpui'
import { Context } from '@neoworks/extension-system'
import type { Plugin } from '@neoworks/extension-system'
import { Bus } from './host/bus.js'
import { ViewRegistry } from './host/plugins/views.js'
import { runtimePlugin } from './host/plugins/runtime.js'
import { configPlugin } from './host/plugins/config.js'
import { unixSocketPlugin } from './host/plugins/unixSocket.js'
import { extensionsPlugin } from './host/plugins/extensions.js'
import { viewTreePlugin } from './host/plugins/viewTree.js'
import { environmentOf, useShellEnvironment } from './system/environment.js'
import { environment, homeDirectory, runtimeDirectory } from './system/fs.js'
import Shell from './Shell.svelte'

// neoshell: one QuickJS process holding the kernel, the bus, every extension
// backend and every view. A view is a Svelte component drawn by the GPUI host,
// and a surface is a layer-shell window.
//
// The kernel boots beside the shell component rather than inside it: its
// mounts are promises, so everything the extensions publish arrives while the
// shell is already running, and the windows fill in as their view types
// register.

const bus = new Bus()
const registry = new ViewRegistry()

// Programs the shell runs. `setsid` launches the user's own applications, so
// it is as wide as the .desktop files the user installed.
const PROGRAMS = [
  'nmcli',
  'bluetoothctl',
  'brightnessctl',
  'playerctl',
  'wpctl',
  'pactl',
  'systemctl',
  'loginctl',
  'hyprctl',
  'busctl',
  'gdbus',
  'getent',
  'gio',
  'setsid',
  'neoshell-windowcapture',
  '/usr/bin/unix_chkpwd',
]

start(Shell, { bus, registry }, {
  window: false,
  process: { allow: PROGRAMS },
  sockets: { allow: ['$runtime/hypr', '$runtime/neoshell-host.sock', '$runtime/neoshell-port'] },
})

void boot()

async function boot(): Promise<void> {
  const shellEnvironment = environmentOf(await environment(), await homeDirectory(), await runtimeDirectory())
  useShellEnvironment(shellEnvironment)
  bus.retain('shell.environment', shellEnvironment)
  const root = new Context()
  mount(root, runtimePlugin, { bus, registry })
  mount(root, configPlugin, { path: shellEnvironment.configPath })
  mount(root, unixSocketPlugin, { socketPath: shellEnvironment.socketPath })
  mount(root, extensionsPlugin)
  mount(root, viewTreePlugin)
}

// A fiber is thenable and rejects when its plugin's apply throws. Nothing here
// awaits one, because a runtime plugin that fails must not stop the rest, so
// the rejection is named rather than left to surface as a bare
// unhandled-promise warning.
function mount(root: Context, plugin: Plugin.Object<any>, config?: unknown): void {
  const fiber = root.plugin(plugin as Plugin.Object, config)
  void Promise.resolve(fiber).catch((error: unknown) => {
    console.error(`neoshell: plugin "${plugin.name}" failed to mount:`, error)
  })
}
