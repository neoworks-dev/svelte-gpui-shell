<script lang="ts">
  import { View } from '@neoworks/svelte-gpui/ui'
  import type { BusService } from '../../lib/bus'
  import type { ViewRegistry } from '../../host/plugins/views'
  import { MAX_UNIT_PX, sizePx, spanOf } from './freeform'

  // A gallery tile showing the real widget, mounted at its design size and
  // scaled down to fit. Previews are live because a widget's whole point is the
  // data in it — a greyed-out shape says nothing about which one to pick.
  //
  // The preview mounts under a "preview:" id, so anything a widget keys by
  // instance (its own settings, its own topic) stays separate from the cards
  // actually on the desktop.
  //
  // Scale works about the card's centre, so the card is shifted back by half
  // of what scaling moved it, and the wrapper clips to what the card occupies.

  let {
    bus,
    registry,
    generation,
    type,
    size,
    width,
  }: {
    bus: BusService
    registry: ViewRegistry
    generation: number
    type: string
    size: string
    width: number
  } = $props()

  const Card = $derived(viewFor(type, generation))
  const box = $derived(sizePx(MAX_UNIT_PX, spanOf(size)))
  const scale = $derived(width / box.width)
  const shiftX = $derived((box.width * (1 - scale)) / 2)
  const shiftY = $derived((box.height * (1 - scale)) / 2)

  function viewFor(widgetType: string, _generation: number) {
    return registry.resolve(widgetType)
  }
</script>

<View style={{ width, height: Math.round(box.height * scale), radius: 24, overflow: 'hidden' }}>
  {#if Card !== undefined}
    <View
      style={{
        position: 'absolute',
        left: -shiftX,
        top: -shiftY,
        width: box.width,
        height: box.height,
        scale,
      }}
    >
      <Card
        {bus}
        {registry}
        args={{ size, width: box.width, height: box.height }}
        id={`preview:${type}`}
      />
    </View>
  {/if}
</View>
