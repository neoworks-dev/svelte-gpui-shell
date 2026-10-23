<script lang="ts">
  import { Button, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { ViewProps } from '../../host/plugins/views'
  import { clockTime, shortDate } from '../../lib/dateFormat'
  import { alpha, colors } from '../../theme'

  // The bar clock. Clicking it toggles the quick settings tray, which is a
  // sibling top-level view: the bar only announces the intent.

  let { bus }: ViewProps = $props()

  let now = $state(new Date())

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date()
    }, 1000)
    return () => {
      clearInterval(timer)
    }
  })

  function toggleQuickSettings(): void {
    bus.publish('quicksettings:toggle', {})
  }
</script>

<Tooltip text="Quick settings" side="bottom" align="end">
  <Button
    style={{ paddingX: 8, paddingY: 2, radius: 8, transition: 150 }}
    hover={{ background: alpha(colors.content, 0.1) }}
    onpress={toggleQuickSettings}
  >
    <View style={{ direction: 'row', gap: 8 }}>
      <Text style={{ color: alpha(colors.content, 0.6) }}>{shortDate(now)}</Text>
      <Text>{clockTime(now)}</Text>
    </View>
  </Button>
</Tooltip>
