import type { Plugin } from '@neoworks/extension-system'
import { requireService } from '../../lib/services.js'
import type { ViewRegistry } from '../../host/plugins/views.js'
import Topbar from './Topbar.svelte'
import Clock from './Clock.svelte'
import Battery from './Battery.svelte'

// neoshell default views — the neoworks profile's surface set. Registration is
// an effect, so a view type leaves the registry with the extension and the
// shell drops the window that was showing it.

const plugin: Plugin.Object = {
  name: 'neoshell-views',
  inject: ['ui'],
  apply(context) {
    const ui = requireService<ViewRegistry>(context, 'ui')
    context.effect(() => ui.register('neoshell.topbar', Topbar))
    context.effect(() => ui.register('neoshell.clock', Clock))
    context.effect(() => ui.register('neoshell.battery', Battery))
  },
}

export default plugin
