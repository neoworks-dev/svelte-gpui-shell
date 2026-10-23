<script lang="ts">
  import { Text, View } from '@neoworks/svelte-gpui/ui'
  import type { ViewProps } from './host/plugins/views'
  import { cardStyle, hintStyle, titleStyle } from './cardLook'

  // A stand-in widget for the smoke harness: the gallery previews whatever the
  // registry holds for a catalog entry's type, and a preview of nothing would
  // build none of the scaling the real one goes through.

  let { id }: ViewProps = $props()
</script>

<View style={cardStyle}>
  <Text style={titleStyle}>Smoke card</Text>
  <Text style={hintStyle}>{id}</Text>
</View>
