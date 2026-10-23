<script lang="ts">
  import { Meter, Text, View } from '@neoworks/svelte-gpui/ui'
  import { dayHighStyle, dayLabelStyle, stripLabelStyle } from '../../cardLook'
  import { alpha, colors } from '../../theme'
  import { bandColor } from './bands'
  import { bandOf } from './lib'
  import type { PollutantEntry } from './lib'

  // One row per pollutant: its concentration, and how much of the WHO guideline
  // that is as a bar. The bars share the card's scale bands, so a row that
  // approaches its guideline reddens the way the index above it does.

  let { pollutants }: { pollutants: PollutantEntry[] } = $props()

  // The concentrations share a unit, so it belongs over the column once rather
  // than on every row.
  const unit = $derived(unitOf(pollutants))

  function unitOf(entries: PollutantEntry[]): string {
    if (entries.length === 0) {
      return ''
    }
    return entries[0].unit
  }
</script>

<View style={{ flex: 1 }}>
  <View style={{ direction: 'row', gap: 8 }}>
    <Text style={{ ...stripLabelStyle, flex: 1 }}>Pollutants</Text>
    <Text style={{ ...stripLabelStyle, width: 52, textAlign: 'right' }}>{unit}</Text>
  </View>
  {#each pollutants as pollutant (pollutant.label)}
    <View style={{ flex: 1, direction: 'row', align: 'center', gap: 8 }}>
      <Text style={{ ...dayLabelStyle, width: 44, truncate: true }}>{pollutant.label}</Text>
      <Meter
        value={pollutant.share}
        min={0}
        max={1}
        color={bandColor(bandOf(pollutant.share))}
        duration={0}
        style={{ flex: 1, height: 4, radius: 2, background: alpha(colors.content, 0.15) }}
        fillStyle={{ radius: 2 }}
      />
      <Text style={{ ...dayHighStyle, width: 52, textAlign: 'right' }}>{pollutant.value}</Text>
    </View>
  {/each}
</View>
