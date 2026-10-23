<script lang="ts">
  import { Tween } from 'svelte/motion'
  import { Icon } from '@neoworks/svelte-gpui/ui'
  import { cubicBezier } from '../../lib/easing'
  import { sampleKeyframes } from '../../lib/keyframes'
  import type { Keyframe } from '../../lib/keyframes'
  import type { Magnification } from './dockModel'

  // One launcher icon: it swells with the pointer's neighbourhood and hops while
  // its app starts. The transform is on the icon rather than the tile, so the
  // row's hit boxes stay put while the icons swell: a magnified tile that moved
  // under the cursor would flip the hover to its neighbour and oscillate.

  const MAGNIFY_MS = 160
  const HOP_MS = 1150

  // Launch feedback: the icon presses into the dock, is flung up with the
  // stretch of leaving the floor, hangs at the apex, then lands squashed. The
  // squash is a uniform scale here, since a style scales both axes together.
  const HOP: Keyframe[] = [
    { at: 0, values: { lift: 0, scale: 1 } },
    { at: 0.09, values: { lift: 0, scale: 0.96 } },
    { at: 0.19, values: { lift: 8, scale: 1.02 } },
    { at: 0.5, values: { lift: 44, scale: 1 } },
    { at: 0.76, values: { lift: 0, scale: 0.97 } },
    { at: 0.86, values: { lift: 0, scale: 1 } },
    { at: 1, values: { lift: 0, scale: 1 } },
  ]
  const HOP_EASING = cubicBezier(0.42, 0, 0.58, 1)

  let {
    icon,
    size,
    magnification,
    hopping,
  }: { icon: string; size: number; magnification: Magnification; hopping: boolean } = $props()

  const swell = new Tween<Magnification>(
    { scale: 1, shift: 0 },
    { duration: MAGNIFY_MS, easing: cubicBezier(0.2, 0.7, 0.3, 1) },
  )
  let hop = $state({ lift: 0, scale: 1 })

  $effect(() => {
    void swell.set(magnification)
  })

  $effect(() => {
    if (!hopping) {
      hop = { lift: 0, scale: 1 }
      return
    }
    const started = performance.now()
    let handle = requestAnimationFrame(function step(now) {
      const progress = ((now - started) % HOP_MS) / HOP_MS
      const frame = sampleKeyframes(HOP, HOP_EASING(progress))
      hop = { lift: frame.lift, scale: frame.scale }
      handle = requestAnimationFrame(step)
    })
    return () => cancelAnimationFrame(handle)
  })
</script>

<Icon
  name={icon}
  fallback="application-x-executable"
  {size}
  style={{ scale: swell.current.scale * hop.scale, translate: [swell.current.shift, -hop.lift] }}
/>
