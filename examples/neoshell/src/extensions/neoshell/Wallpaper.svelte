<script lang="ts">
  import type { ViewProps } from '../../host/plugins/views'
  import Surface from '../../Surface.svelte'
  import { colors } from '../../theme'
  import WallpaperLayer from './WallpaperLayer.svelte'

  // The desktop background window: the configured wallpaper over a flat dark
  // fill, which is all there is when none is set.

  let { bus, args }: ViewProps = $props()
</script>

<Surface {args} style={{ background: colors.surface }}>
  <WallpaperLayer {bus} />
</Surface>
