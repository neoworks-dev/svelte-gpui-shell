<script lang="ts">
  import { View } from '@neoworks/svelte-gpui/ui'
  import { colors } from '../../theme'

  // Four bars bouncing in sequence: the "audio is coming out of here" mark on
  // the island's right edge. A paused player freezes them mid-stride.

  const BAR_DELAYS_MS = [0, 130, 260, 390]
  const HALF_PERIOD_MS = 900
  const LOWEST_SCALE = 0.3

  let { large, playing }: { large: boolean; playing: boolean } = $props()

  let elapsed = $state(0)

  $effect(() => {
    if (!playing) {
      return
    }
    let previous = performance.now()
    let handle = requestAnimationFrame(function step(now) {
      elapsed += now - previous
      previous = now
      handle = requestAnimationFrame(step)
    })
    return () => cancelAnimationFrame(handle)
  })

  const height = $derived(heightOf(large))

  function heightOf(isLarge: boolean): number {
    if (isLarge) {
      return 16
    }
    return 12
  }

  // A triangle wave per bar, like an alternating ease between the two scales.
  function scaleOf(delay: number): number {
    const phase = Math.max(0, elapsed - delay) / HALF_PERIOD_MS
    const position = phase % 2
    let rise = position
    if (position > 1) {
      rise = 2 - position
    }
    return LOWEST_SCALE + (1 - LOWEST_SCALE) * rise
  }
</script>

<View style={{ direction: 'row', align: 'center', gap: 2, height }}>
  {#each BAR_DELAYS_MS as delay (delay)}
    <View style={{ width: 2, height: height * scaleOf(delay), radius: 1, background: colors.accent }}></View>
  {/each}
</View>
