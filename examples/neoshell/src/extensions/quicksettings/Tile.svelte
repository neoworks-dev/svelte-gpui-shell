<script lang="ts">
  import { Button, Icon, Text, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import { alpha, colors } from '../../theme'
  import { iconName } from './icons'

  // One quick-settings tile. A wide tile shows icon, title and subtitle in a
  // row; a compact tile stacks the icon over a label. Tiles that drill into a
  // detail page carry a chevron that is its own button, so toggling and
  // opening stay separate gestures.

  const TILE_HEIGHT = 56
  const BADGE_SIZE = 32
  const COMPACT_BADGE_SIZE = 26

  let {
    icon,
    title,
    subtitle = '',
    active = false,
    disabled = false,
    compact = false,
    onActivate,
    onOpen,
  }: {
    icon: string
    title: string
    subtitle?: string
    active?: boolean
    disabled?: boolean
    compact?: boolean
    onActivate?: () => void
    onOpen?: () => void
  } = $props()

  const surface = $derived(surfaceOf(active, disabled))
  const surfaceHover = $derived(surfaceHoverOf(active, disabled))
  const badge = $derived(badgeOf(active))

  function surfaceOf(isActive: boolean, isDisabled: boolean): Style {
    if (isDisabled) {
      return { background: alpha(colors.surface, 0.5), opacity: 0.45 }
    }
    if (isActive) {
      return { background: alpha(colors.info, 0.2) }
    }
    return { background: alpha(colors.surface, 0.8) }
  }

  function surfaceHoverOf(isActive: boolean, isDisabled: boolean): Style {
    if (isDisabled) {
      return {}
    }
    if (isActive) {
      return { background: alpha(colors.info, 0.25) }
    }
    return { background: colors.surface }
  }

  function badgeOf(isActive: boolean): Style {
    if (isActive) {
      return { background: colors.info, color: colors.canvas }
    }
    return { background: alpha(colors.content, 0.1), color: alpha(colors.content, 0.7) }
  }

  function activate(): void {
    if (disabled || onActivate === undefined) {
      return
    }
    onActivate()
  }

  function open(): void {
    if (disabled || onOpen === undefined) {
      return
    }
    onOpen()
  }
</script>

<!-- TODO(host: popups) the compact tile's title tooltip and the chevron's "Open … settings" tooltip. -->
{#if compact}
  <Button
    style={{
      ...surface,
      flex: 1,
      height: TILE_HEIGHT,
      radius: 16,
      direction: 'column',
      align: 'center',
      justify: 'center',
      gap: 2,
      transition: 120,
    }}
    hover={surfaceHover}
    {disabled}
    onpress={activate}
  >
    <View
      style={{
        ...badge,
        width: COMPACT_BADGE_SIZE,
        height: COMPACT_BADGE_SIZE,
        radius: COMPACT_BADGE_SIZE / 2,
        align: 'center',
        justify: 'center',
      }}
    >
      <Icon name={iconName(icon)} size={15} />
    </View>
    <Text style={{ fontSize: 10, color: colors.content, textAlign: 'center', maxLines: 2, maxWidth: 56 }}>{title}</Text>
  </Button>
{:else}
  <View
    style={{
      ...surface,
      flex: 1,
      height: TILE_HEIGHT,
      radius: 16,
      direction: 'row',
      overflow: 'hidden',
      transition: 120,
    }}
    hover={surfaceHover}
  >
    <Button
      style={{ flex: 1, minWidth: 0, direction: 'row', align: 'center', gap: 10, paddingLeft: 10 }}
      {disabled}
      onpress={activate}
    >
      <View
        style={{
          ...badge,
          width: BADGE_SIZE,
          height: BADGE_SIZE,
          radius: BADGE_SIZE / 2,
          align: 'center',
          justify: 'center',
        }}
      >
        <Icon name={iconName(icon)} size={17} />
      </View>
      <View style={{ flex: 1, minWidth: 0, justify: 'center' }}>
        <Text style={{ fontSize: 12, fontWeight: 500, color: colors.content, truncate: true }}>{title}</Text>
        {#if subtitle !== ''}
          <Text style={{ fontSize: 10, color: alpha(colors.content, 0.55), truncate: true }}>{subtitle}</Text>
        {/if}
      </View>
    </Button>
    {#if onOpen !== undefined}
      <Button
        style={{ paddingLeft: 4, paddingRight: 8, align: 'center', color: alpha(colors.content, 0.45), transition: 120 }}
        hover={{ color: colors.content }}
        {disabled}
        onpress={open}
      >
        <Icon name={iconName('chevronRight')} size={15} />
      </Button>
    {/if}
  </View>
{/if}
