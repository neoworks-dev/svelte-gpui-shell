<script lang="ts">
  import { View } from '@neoworks/svelte-gpui/ui'
  import { colors } from '../../theme'

  // The inverted corner that makes the bar curve into the notch: canvas black
  // with a quarter circle cut out of the corner that faces away from the island.
  // The circle is centred on the bottom corner nearest the bar's cutout, with
  // the fillet's own size as its radius, and its edge is one pixel of fade.

  const SIZE = 10
  const EDGE_FADE = 0.5 / SIZE
  const CLEAR = `${colors.canvas}00`

  let { side }: { side: 'left' | 'right' } = $props()

  const CIRCLE_CENTER_X = { left: 0, right: 1 }
</script>

<View
  style={{
    width: SIZE,
    height: SIZE,
    background: {
      radial: {
        center: [CIRCLE_CENTER_X[side], 1],
        radius: 1,
        stops: [CLEAR, [CLEAR, 1 - EDGE_FADE], [colors.canvas, 1 + EDGE_FADE]],
      },
    },
  }}
></View>
