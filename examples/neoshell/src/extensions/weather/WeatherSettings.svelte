<script lang="ts">
  import { untrack } from 'svelte'
  import { Button, Input, Text, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import { alpha, colors } from '../../theme'

  // The card's own settings, per widget instance. Two weather widgets on one
  // desktop are two places, so this writes weather.cards.<id> rather than the
  // shared section: a card that has never been edited keeps following the
  // section's defaults.
  //
  // Typing into a card on the background layer only works because the desktop's
  // view-tree node asks for keyboard: ondemand; a layer that takes no focus
  // cannot be typed into at all.

  let {
    place,
    imperial,
    onsave,
    oncancel,
  }: {
    place: string
    imperial: boolean
    onsave: (place: string, imperial: boolean) => void
    oncancel: () => void
  } = $props()

  // Seeded once, on purpose: the form is mounted when the card flips to its
  // settings face, and a poll landing mid-edit must not overwrite what is being
  // typed.
  let draftPlace = $state(untrack(() => place))
  let draftImperial = $state(untrack(() => imperial))

  const labelStyle: Style = { fontSize: 11, color: alpha(colors.content, 0.6) }
  const buttonStyle: Style = {
    fontSize: 11,
    paddingX: 8,
    paddingY: 2,
    radius: 8,
    borderWidth: 1,
    borderColor: alpha(colors.content, 0.15),
    align: 'center',
    justify: 'center',
    transition: 120,
  }
  const buttonHover: Style = { background: alpha(colors.content, 0.1) }

  function submit(): void {
    onsave(draftPlace.trim(), draftImperial)
  }

  function unitLabel(isImperial: boolean): string {
    if (isImperial) {
      return '°F'
    }
    return '°C'
  }
</script>

<View style={{ flex: 1, gap: 8 }}>
  <View style={{ gap: 2 }}>
    <Text style={labelStyle}>Location</Text>
    <Input
      bind:value={draftPlace}
      placeholder="City"
      style={{
        fontSize: 13,
        paddingX: 8,
        paddingY: 2,
        radius: 8,
        background: alpha('#000000', 0.6),
        borderWidth: 1,
        borderColor: alpha(colors.content, 0.15),
      }}
      focus={{ borderColor: alpha(colors.content, 0.4) }}
      onsubmit={submit}
    />
  </View>

  <View style={{ direction: 'row', align: 'center', gap: 8 }}>
    <Text style={{ ...labelStyle, flex: 1 }}>Units</Text>
    <Button style={buttonStyle} hover={buttonHover} onpress={() => (draftImperial = !draftImperial)}>
      <Text>{unitLabel(draftImperial)}</Text>
    </Button>
  </View>

  <View style={{ flex: 1, direction: 'row', gap: 8, align: 'end' }}>
    <Button style={{ ...buttonStyle, flex: 1 }} hover={buttonHover} onpress={oncancel}>
      <Text>Cancel</Text>
    </Button>
    <Button
      style={{ ...buttonStyle, flex: 1, background: colors.content, color: colors.canvas, borderColor: 'transparent' }}
      hover={{ background: alpha(colors.content, 0.85) }}
      onpress={submit}
    >
      <Text>Done</Text>
    </Button>
  </View>
</View>
