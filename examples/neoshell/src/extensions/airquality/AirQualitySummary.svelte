<script lang="ts">
  import { Text, View } from '@neoworks/svelte-gpui/ui'
  import {
    conditionStyle,
    placeStyle,
    rangeStyle,
    readingStyle,
    titleStyle,
  } from '../../cardLook'
  import { colors } from '../../theme'
  import { clamped } from './lib'
  import type { AirQualityCurrent } from './lib'

  // The index, its band and the position on the scale, in the two shapes the
  // card sizes need: stacked on a 2x2 card, and side by side on the wider ones,
  // where the strips below claim the lower half.

  const AGE_TICK_MS = 30_000
  const MINUTE_MS = 60_000
  const HOUR_MS = 60 * MINUTE_MS
  const KNOB_PX = 14
  const TRACK_HEIGHT = 8
  const RAMP_STOPS: [string, string, string, string] = [
    colors.success,
    colors.warning,
    colors.error,
    colors.accent,
  ]

  let {
    current,
    stacked,
    trackWidth,
  }: { current: AirQualityCurrent; stacked: boolean; trackWidth: number } = $props()

  let now = $state(Date.now())

  const fraction = $derived(clamped(current.index / current.max))
  const age = $derived(describeAge(now - current.updatedAt))
  // The knob is centred on the reading, so it starts half its width earlier
  // and the track keeps it inside at both ends.
  const knobLeft = $derived(
    Math.round(Math.min(Math.max(0, fraction * trackWidth - KNOB_PX / 2), trackWidth - KNOB_PX)),
  )

  // The card states how old its reading is, so it has to re-render on a timer
  // even when nothing new arrives.
  $effect(() => {
    const timer = setInterval(() => {
      now = Date.now()
    }, AGE_TICK_MS)
    return () => {
      clearInterval(timer)
    }
  })

  function describeAge(elapsed: number): string {
    if (elapsed < MINUTE_MS) {
      return 'just now'
    }
    if (elapsed < HOUR_MS) {
      return `${Math.floor(elapsed / MINUTE_MS)} minutes ago`
    }
    return `${Math.floor(elapsed / HOUR_MS)} hours ago`
  }
</script>

<!-- The knob sits over the ramp at the reading. -->
{#snippet scaleBar()}
  <View style={{ height: KNOB_PX, justify: 'center' }}>
    <View
      style={{
        width: trackWidth,
        height: TRACK_HEIGHT,
        radius: 4,
        background: { linear: { angle: 90, stops: RAMP_STOPS } },
      }}
    ></View>
    <View
      style={{
        position: 'absolute',
        left: knobLeft,
        top: 0,
        size: KNOB_PX,
        radius: KNOB_PX / 2,
        background: colors.content,
        borderWidth: 2,
        borderColor: colors.canvas,
      }}
    ></View>
  </View>
{/snippet}

{#if stacked}
  <View style={{ flex: 1, gap: 6 }}>
    <Text style={titleStyle}>Air Quality</Text>
    <Text style={readingStyle}>{current.index}</Text>
    <View style={{ flex: 1, justify: 'end' }}>
      <Text style={conditionStyle}>{current.category}</Text>
      <Text style={rangeStyle}>{age}</Text>
    </View>
    {@render scaleBar()}
  </View>
{:else}
  <View style={{ gap: 8 }}>
    <View style={{ direction: 'row', gap: 12 }}>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text style={titleStyle}>Air Quality</Text>
        <Text style={readingStyle}>{current.index}</Text>
      </View>
      <View style={{ align: 'end' }}>
        <Text style={placeStyle}>{current.place}</Text>
        <Text style={conditionStyle}>{current.category}</Text>
        <Text style={rangeStyle}>{age}</Text>
      </View>
    </View>
    {@render scaleBar()}
  </View>
{/if}
