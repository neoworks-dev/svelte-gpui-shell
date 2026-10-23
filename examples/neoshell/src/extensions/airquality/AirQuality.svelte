<script lang="ts">
  import { subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { Text, View } from '@neoworks/svelte-gpui/ui'
  import { cardStyle, hintStyle, titleStyle } from '../../cardLook'
  import { widgetBoxOf } from '../../lib/widget'
  import AirQualitySummary from './AirQualitySummary.svelte'
  import IndexStrip from './IndexStrip.svelte'
  import PollutantList from './PollutantList.svelte'
  import { currentOf } from './lib'
  import type { AirQualityCurrent } from './lib'

  // The desktop air quality card in the three widget sizes:
  //
  //   small (2x2)   the index, its band and the position on the scale
  //   medium (4x2)  the same laid out wide, plus the next hours
  //   large (4x4)   the same, plus the pollutants behind the index
  //
  // The size is a prop rather than something the card measures — see the
  // weather card for why the webview build had to observe its own box and this
  // one does not.
  //
  // One topic for the whole extension, not one per card: unlike weather, the
  // backend keeps a single location, so two air quality widgets show the same
  // reading.

  // What the scale bar has left once the card's padding has taken its share.
  const CARD_PADDING_PX = 28

  let { bus, args }: ViewProps = $props()

  let current = $state<AirQualityCurrent | null>(null)

  const box = $derived(widgetBoxOf(args))
  const trackWidth = $derived(Math.max(40, box.width - CARD_PADDING_PX))

  $effect(() =>
    subscribeTo(bus, 'airquality.current', (message) => {
      current = currentOf(message.data, Date.now())
    }),
  )


</script>

<View style={cardStyle}>
  {#if current === null}
    <View style={{ gap: 4 }}>
      <Text style={titleStyle}>Air Quality</Text>
      <Text style={hintStyle}>Set airquality.place in the neoshell config.</Text>
    </View>
  {:else if box.size === 'small'}
    <AirQualitySummary {current} stacked={true} {trackWidth} />
  {:else if box.size === 'medium'}
    <AirQualitySummary {current} stacked={false} {trackWidth} />
    <View style={{ flex: 1 }}></View>
    <IndexStrip hours={current.hours} max={current.max} />
  {:else}
    <AirQualitySummary {current} stacked={false} {trackWidth} />
    <IndexStrip hours={current.hours} max={current.max} />
    <PollutantList pollutants={current.pollutants} />
  {/if}
</View>
