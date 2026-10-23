<script lang="ts">
  import { Canvas } from '@neoworks/svelte-gpui/ui'
  import { colors } from '../../theme'

  // The inverted corner that makes the bar curve into the notch: a square of
  // canvas black with a quarter circle cut out of the corner that faces away
  // from the island. Drawn row by row as the circle's chord, because a fill
  // cannot be cut out.
  //
  // TODO(host: radial gradients) a radial-gradient background would replace the canvas.

  const SIZE = 10

  let { side }: { side: 'left' | 'right' } = $props()

  let canvas: Canvas | undefined = $state()

  $effect(() => {
    if (canvas !== undefined) {
      draw(canvas)
    }
  })

  // The circle is centred on the bottom corner nearest the bar's cutout, with
  // the fillet's own size as its radius; everything outside it is black.
  function draw(target: Canvas): void {
    target.clear()
    for (let row = 0; row < SIZE; row += 1) {
      const rise = row + 0.5 - SIZE
      const reach = Math.sqrt(SIZE * SIZE - rise * rise)
      if (side === 'left') {
        target.fillRect(reach, row, SIZE - reach, 1, colors.canvas)
      } else {
        target.fillRect(0, row, SIZE - reach, 1, colors.canvas)
      }
    }
  }
</script>

<Canvas bind:this={canvas} style={{ width: SIZE, height: SIZE }} />
