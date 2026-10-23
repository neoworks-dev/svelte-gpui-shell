<script lang="ts">
  import { Icon, Text, View } from '@neoworks/svelte-gpui/ui'
  import {
    conditionStyle,
    placeStyle,
    rangeStyle,
    readingStyle,
  } from '../../cardLook'
  import { NO_TEMPERATURE, weatherIcon } from './lib'
  import type { WeatherCurrent } from './lib'

  // The place, the temperature and the condition, in the two shapes the card
  // sizes need: stacked with the condition at the foot on a 2x2 card, and side
  // by side on the wider ones, where the strips below claim the lower half.

  const ICON_LARGE = 36
  const ICON_SMALL = 32

  let { current, stacked }: { current: WeatherCurrent; stacked: boolean } = $props()

  const showsRange = $derived(current.high !== NO_TEMPERATURE && current.low !== NO_TEMPERATURE)
  const icon = $derived(weatherIcon(current.code, current.isDay))

  const weatherRangeStyle = { ...rangeStyle, tabularNumbers: true }
</script>

{#if stacked}
  <View style={{ flex: 1, gap: 6 }}>
    <Text style={placeStyle}>{current.place}</Text>
    <Text style={readingStyle}>{current.temperature}{current.unit}</Text>
    <View style={{ flex: 1, gap: 2, justify: 'end' }}>
      <Icon name={icon} size={ICON_LARGE} />
      <Text style={conditionStyle}>{current.description}</Text>
      {#if showsRange}
        <Text style={weatherRangeStyle}>H:{current.high}° L:{current.low}°</Text>
      {/if}
    </View>
  </View>
{:else}
  <View style={{ direction: 'row', gap: 12 }}>
    <View style={{ flex: 1, minWidth: 0 }}>
      <Text style={placeStyle}>{current.place}</Text>
      <Text style={readingStyle}>{current.temperature}{current.unit}</Text>
    </View>
    <View style={{ gap: 2, align: 'end' }}>
      <Icon name={icon} size={ICON_SMALL} />
      <Text style={conditionStyle}>{current.description}</Text>
      {#if showsRange}
        <Text style={weatherRangeStyle}>H:{current.high}° L:{current.low}°</Text>
      {/if}
    </View>
  </View>
{/if}
