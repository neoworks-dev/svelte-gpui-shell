<script lang="ts">
  import { Button, Icon, Slider, View } from '@neoworks/svelte-gpui/ui'
  import { alpha, colors } from '../../theme'
  import { iconName } from './icons'

  // A quick-settings slider: one rounded pill whose fill is the value, with
  // the icon sitting inside the left end. The value is committed while
  // dragging, so audio and backlight follow the pointer; the backend
  // republishes and the parent feeds the value back in.

  const ROW_HEIGHT = 32
  const HIDDEN_KNOB = 2
  const ICON_SLOT = 36

  let {
    icon,
    label,
    value,
    disabled = false,
    onInput,
    onIconClick,
  }: {
    icon: string
    label: string
    value: number
    disabled?: boolean
    onInput: (percent: number) => void
    onIconClick?: () => void
  } = $props()

  const iconInteractive = $derived(onIconClick !== undefined && !disabled)

  // The backends take whole percent.
  function handleChange(next: number): void {
    const percent = Math.round(next)
    if (percent === value) {
      return
    }
    onInput(percent)
  }

  function handleIconClick(): void {
    if (onIconClick === undefined || disabled) {
      return
    }
    onIconClick()
  }
</script>

<!-- TODO(host: popups) the slider's label tooltip. -->
<View style={{ height: ROW_HEIGHT }}>
  <Slider
    {value}
    {disabled}
    min={0}
    max={100}
    step={1}
    knobSize={HIDDEN_KNOB}
    onchange={handleChange}
    style={{ height: ROW_HEIGHT }}
    trackStyle={{ height: ROW_HEIGHT, radius: 16, background: alpha('#ffffff', 0.1) }}
    fillStyle={{ height: ROW_HEIGHT, radius: 16, background: colors.info }}
    knobStyle={{ opacity: 0, shadow: null, borderWidth: 0 }}
  />

  <!-- The icon sits over the left end of the trough. It is only a button when
       it does something: a mute toggle is clickable, a brightness lamp is not,
       and a button that swallowed the click would leave that end of the
       slider unreachable. -->
  {#if iconInteractive}
    <Button
      style={{
        position: 'absolute',
        left: 0,
        top: 0,
        width: ICON_SLOT,
        height: ROW_HEIGHT,
        radius: 16,
        align: 'center',
        justify: 'center',
        color: alpha(colors.content, 0.8),
      }}
      hover={{ color: colors.content }}
      onpress={handleIconClick}
    >
      <Icon name={iconName(icon)} size={17} />
    </Button>
  {:else}
    <View
      style={{
        position: 'absolute',
        left: 0,
        top: 0,
        width: ICON_SLOT,
        height: ROW_HEIGHT,
        align: 'center',
        justify: 'center',
        color: alpha(colors.content, 0.8),
      }}
    >
      <Icon name={iconName(icon)} size={17} />
    </View>
  {/if}
</View>
