<script lang="ts">
  import { Icon, Text, View } from '@neoworks/svelte-gpui/ui'
  import bolt from '@phosphor-icons/core/fill/lightning-fill.svg'
  import { subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { recordOf } from '../../lib/record'
  import { alpha, colors } from '../../theme'

  // Battery indicator: an outline with a level fill and a bolt while charging.
  // Renders nothing until system.battery delivers data (desktops without a
  // battery never publish it).

  const TRACK_WIDTH = 18

  let { bus }: ViewProps = $props()

  let percent = $state(-1)
  let charging = $state(false)

  $effect(() =>
    subscribeTo(bus, 'system.battery', (message) => {
      const data = recordOf(message.data)
      if (typeof data.percent === 'number') {
        percent = Math.round(data.percent)
      }
      charging = data.charging === true
    }),
  )

  const fillWidth = $derived(Math.max(2, Math.round((percent / 100) * TRACK_WIDTH)))
  const fillColor = $derived(fillColorOf(percent, charging))

  function fillColorOf(level: number, isCharging: boolean): string {
    if (level >= 0 && level <= 15 && !isCharging) {
      return colors.error
    }
    return alpha(colors.content, 0.8)
  }
</script>

{#if percent >= 0}
  <View style={{ direction: 'row', align: 'center', gap: 6, color: alpha(colors.content, 0.8) }}>
    <Text style={{ tabularNumbers: true }}>{percent}%</Text>
    <View
      style={{
        width: 22,
        height: 13,
        padding: 1,
        borderWidth: 1,
        borderColor: alpha(colors.content, 0.5),
        radius: 4,
        justify: 'center',
      }}
    >
      <View style={{ width: fillWidth, height: 9, radius: 2, background: fillColor }}></View>
      {#if charging}
        <View style={{ position: 'absolute', inset: 0, align: 'center', justify: 'center' }}>
          <Icon svg={bolt} size={9} color={colors.canvas} />
        </View>
      {/if}
    </View>
    <View
      style={{ width: 2, height: 5, background: alpha(colors.content, 0.5), radius: [0, 1, 1, 0] }}
    ></View>
  </View>
{/if}
