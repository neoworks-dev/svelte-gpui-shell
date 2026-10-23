<script lang="ts">
  import { Icon, Text, View } from '@neoworks/svelte-gpui/ui'
  import { dayHighStyle, dayLabelStyle } from '../../cardLook'
  import { alpha, colors } from '../../theme'
  import { weatherIcon } from './lib'
  import type { DayEntry } from './lib'

  // One row per forecast day: the weekday, its condition, and the day's range
  // as a bar scaled against the whole list, so the rows read against each other
  // rather than each against itself.

  // A day whose high and low round to the same number would otherwise draw a
  // zero-width bar.
  const MIN_BAR_PX = 8
  const ICON_PIXELS = 20

  let { days, trackWidth }: { days: DayEntry[]; trackWidth: number } = $props()

  const scale = $derived(scaleOf(days))

  function scaleOf(entries: DayEntry[]): { low: number; span: number } {
    if (entries.length === 0) {
      return { low: 0, span: 0 }
    }
    const low = Math.min(...entries.map((entry) => entry.low))
    const high = Math.max(...entries.map((entry) => entry.high))
    return { low, span: high - low }
  }

  // Widening a flat day's bar would push it past the track when that day sits
  // at the top of the scale, so the bar is pulled back instead.
  function barOf(day: DayEntry): { left: number; width: number } {
    const width = Math.max(MIN_BAR_PX, positionOf(day.high) - positionOf(day.low))
    return { left: Math.round(Math.min(positionOf(day.low), trackWidth - width)), width: Math.round(width) }
  }

  function positionOf(value: number): number {
    if (scale.span <= 0) {
      return 0
    }
    return ((value - scale.low) / scale.span) * trackWidth
  }
</script>

<View style={{ flex: 1 }}>
  {#each days as day (day.label)}
    {@const bar = barOf(day)}
    <View style={{ flex: 1, direction: 'row', align: 'center', gap: 8 }}>
      <Text style={{ ...dayLabelStyle, width: 36, truncate: true }}>{day.label}</Text>
      <Icon name={weatherIcon(day.code, true)} size={ICON_PIXELS} />
      <Text style={{ ...dayLabelStyle, color: alpha(colors.content, 0.6), width: 26, textAlign: 'right', tabularNumbers: true }}>{day.low}°</Text>
      <View style={{ width: trackWidth, height: 4, radius: 2, background: alpha(colors.content, 0.15) }}>
        <View
          style={{
            marginLeft: bar.left,
            width: bar.width,
            height: 4,
            radius: 2,
            background: { linear: { angle: 90, stops: [colors.info, colors.warning] } },
          }}
        ></View>
      </View>
      <Text style={{ ...dayHighStyle, width: 26, textAlign: 'right' }}>{day.high}°</Text>
    </View>
  {/each}
</View>
