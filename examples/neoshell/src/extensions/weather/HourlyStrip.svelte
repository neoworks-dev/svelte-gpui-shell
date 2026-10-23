<script lang="ts">
  import { Icon, Text, View } from '@neoworks/svelte-gpui/ui'
  import { stripLabelStyle, stripValueStyle } from '../../cardLook'
  import { weatherIcon } from './lib'
  import type { HourEntry } from './lib'

  // The next few hours as evenly spread columns. The unit symbol stays on the
  // headline temperature above: repeating it six times reads as noise at this
  // size, and the degree sign carries the rest.

  const ICON_PIXELS = 20

  let { hours }: { hours: HourEntry[] } = $props()
</script>

<View style={{ direction: 'row' }}>
  {#each hours as hour (hour.label)}
    <View style={{ flex: 1, gap: 2, align: 'center' }}>
      <Text style={stripLabelStyle}>{hour.label}</Text>
      <Icon name={weatherIcon(hour.code, hour.isDay)} size={ICON_PIXELS} />
      <Text style={stripValueStyle}>{hour.temperature}°</Text>
    </View>
  {/each}
</View>
