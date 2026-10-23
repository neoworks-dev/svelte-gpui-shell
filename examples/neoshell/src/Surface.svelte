<script lang="ts">
  import type { Snippet } from 'svelte'
  import { View, Window } from '@neoworks/svelte-gpui/ui'
  import type { Size, Style } from '@neoworks/svelte-gpui/ui'
  import { valueOr } from './lib/record'
  import { surfaceOf } from './surface'
  import {
    anchorOf,
    exclusiveZoneOf,
    keyboardOf,
    layerOf,
    marginOf,
    recordOf,
    sizeOf,
  } from './tree'

  // The window a top-level view draws in: a layer surface placed by the view
  // tree node's args. Width and height default to the node's, and a view whose
  // window changes size while it runs passes its own. With `plain`, the
  // surface is a view in the window around it, so the smoke harness can lay
  // every view out side by side.

  let {
    args,
    width,
    height,
    exclusiveZone,
    keyboard,
    style,
    children,
  }: {
    args: unknown
    width?: number
    height?: number
    exclusiveZone?: number
    keyboard?: 'none' | 'on-demand' | 'exclusive'
    style?: Style
    children?: Snippet
  } = $props()

  const surface = surfaceOf()
  const record = $derived(recordOf(args))
  const surfaceWidth = $derived(valueOr(width, sizeOf(record, 'width')))
  const surfaceHeight = $derived(valueOr(height, sizeOf(record, 'height')))
  const surfaceZone = $derived(valueOr(exclusiveZone, exclusiveZoneOf(record)))
  const surfaceKeyboard = $derived(valueOr(keyboard, keyboardOf(record)))
  const plainStyle = $derived({
    ...style,
    width: plainSize(surfaceWidth),
    height: plainSize(surfaceHeight),
  })

  // Size 0 stretches a layer surface; in a plain view it fills its parent.
  function plainSize(size: number): Size {
    if (size === 0) {
      return 'full'
    }
    return size
  }
</script>

{#if surface.plain}
  <View style={plainStyle}>
    {@render children?.()}
  </View>
{:else}
  <Window
    monitor={surface.monitor}
    layer={layerOf(record)}
    anchor={anchorOf(record)}
    keyboard={surfaceKeyboard}
    exclusiveZone={surfaceZone}
    margin={marginOf(record)}
    width={surfaceWidth}
    height={surfaceHeight}
    namespace={`neoshell.${surface.name}`}
    {style}
  >
    {@render children?.()}
  </Window>
{/if}
