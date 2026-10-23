<script lang="ts">
  import { Text, View } from '@neoworks/svelte-gpui/ui'
  import { stripLabelStyle, stripValueStyle } from '../../cardLook'
  import { alpha, colors } from '../../theme'
  import { bandColor } from './bands'
  import { bandOf, clamped } from './lib'
  import type { HourEntry } from './lib'

  // The next few hours as columns, each a bar against a common track. Height
  // and colour both carry the index, so the shape of the strip reads before the
  // numbers under it do.

  const TRACK_HEIGHT = 28
  const BAR_WIDTH = 6
  // A clean hour would otherwise draw nothing at all, which reads as a gap in
  // the forecast rather than as good air.
  const MIN_BAR_PX = 3

  let { hours, max }: { hours: HourEntry[]; max: number } = $props()

  function heightOf(hour: HourEntry): number {
    return Math.round(Math.max(MIN_BAR_PX, clamped(hour.index / max) * TRACK_HEIGHT))
  }
</script>

<View style={{ direction: 'row' }}>
  {#each hours as hour (hour.label)}
    <View style={{ flex: 1, gap: 2, align: 'center' }}>
      <Text style={stripLabelStyle}>{hour.label}</Text>
      <View
        style={{
          width: BAR_WIDTH,
          height: TRACK_HEIGHT,
          radius: 3,
          background: alpha(colors.content, 0.15),
          justify: 'end',
        }}
      >
        <View
          style={{ height: heightOf(hour), radius: 3, background: bandColor(bandOf(hour.index / max)) }}
        ></View>
      </View>
      <Text style={stripValueStyle}>{hour.index}</Text>
    </View>
  {/each}
</View>
