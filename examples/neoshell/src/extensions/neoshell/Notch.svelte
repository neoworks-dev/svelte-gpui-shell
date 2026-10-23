<script lang="ts">
  import { Tween } from 'svelte/motion'
  import { Button, Icon, Image, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { Size, Style } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import { cubicBezier } from '../../lib/easing'
  import type { ViewProps } from '../../host/plugins/views'
  import { recordOf } from '../../lib/record'
  import Surface from '../../Surface.svelte'
  import { alpha, colors } from '../../theme'
  import Fillet from './Fillet.svelte'
  import LevelMeter from './LevelMeter.svelte'

  // The notch: a black cutout welded to the top edge, centred over the bar. It
  // renders one island entry at a time: the highest-priority member of a
  // stack fed by island:show / island:dismiss plus the built-in producers
  // below (focused window, volume, media).
  //
  //   island:show    {id, priority?, label, detail?, progress?, tone?, ttlMs?}
  //   island:dismiss {id}
  //
  // Hovering only nudges the island a few pixels wider, to say it is
  // clickable; a click is what opens it.
  //
  // The layer surface is large enough for the open island, its overshoot and
  // the label tooltip; only the island's own box takes the pointer.

  interface IslandEntry {
    id: string
    priority: number
    label: string
    detail: string
    progress: number
    tone: string
    art: string
  }

  interface MediaState {
    status: string
    length: number
    shuffle: boolean
  }

  interface Dimensions {
    width: number
    height: number
  }

  const PRIORITY_IDLE = 0
  const PRIORITY_ACTIVITY = 10
  const PRIORITY_TRANSIENT = 20
  const PRIORITY_ALERT = 30

  const TRANSIENT_TTL_MS = 1600
  const NO_PROGRESS = -1
  const MEDIA_ID = 'media'

  const BAR_HEIGHT = 30
  const FILLET_WIDTH = 10
  const ISLAND_RADIUS = 10
  const OPEN_DURATION_MS = 260

  const SIZES: Record<string, Dimensions> = {
    empty: { width: 128, height: BAR_HEIGHT },
    rest: { width: 272, height: BAR_HEIGHT },
    active: { width: 320, height: BAR_HEIGHT },
    expanded: { width: 380, height: 116 },
    expandedMedia: { width: 380, height: 164 },
  }

  const HOVER_GROWTH: Dimensions = { width: 12, height: 3 }

  const TONE_COLORS: Record<string, string> = {
    neutral: colors.content,
    accent: colors.accent,
    success: colors.success,
    warning: colors.warning,
    error: colors.error,
  }

  const ICONS = {
    shuffle: 'media-playlist-shuffle-symbolic',
    previous: 'media-skip-backward-symbolic',
    next: 'media-skip-forward-symbolic',
    play: 'media-playback-start-symbolic',
    pause: 'media-playback-pause-symbolic',
  }

  const ISLAND_PADDING = 16
  const WINDOW_WIDTH = 460
  const WINDOW_HEIGHT = 240
  const TOOLTIP_OFFSET = 4
  const SCRUB_TIME_WIDTH = 48

  let { bus, args }: ViewProps = $props()

  let entries = $state<IslandEntry[]>([])
  let hovered = $state(false)
  let opened = $state(false)
  let media = $state<MediaState>({ status: '', length: 0, shuffle: false })
  let position = $state(0)

  const dismissTimers = new Map<string, ReturnType<typeof setTimeout>>()
  let lastVolumeLevel = ''
  let lastTrackTitle = ''
  let openedId = ''

  const current = $derived(topEntry(entries))
  const isMedia = $derived(current !== null && current.id === MEDIA_ID)
  const mode = $derived(modeOf(current, opened, isMedia))
  const size = $derived(grownSize(SIZES[mode], mode, hovered))
  const isExpanded = $derived(mode.startsWith('expanded'))

  // Opening moves on a spring-like curve that overshoots a little before it
  // settles; the surface follows every frame.
  const animatedSize = new Tween<Dimensions>(SIZES.empty, {
    duration: OPEN_DURATION_MS,
    easing: cubicBezier(0.32, 1.24, 0.5, 1),
  })

  $effect(() => {
    void animatedSize.set(size)
  })

  const islandWidth = $derived(Math.max(0, Math.round(animatedSize.current.width)))
  const islandHeight = $derived(Math.max(0, Math.round(animatedSize.current.height)))

  $effect(() =>
    subscribeTo(bus, 'island:show', (message) => {
      const shown = entryFromMessage(message.data)
      if (shown === null) {
        return
      }
      show(shown.entry, shown.ttlMs)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'island:dismiss', (message) => {
      dismiss(stringOf(recordOf(message.data).id))
    }),
  )

  // The focused window is the island's resting content: persistent, and the
  // lowest priority, so anything else displaces it while it lasts.
  $effect(() =>
    subscribeTo(bus, 'hypr.activewindow', (message) => {
      const record = recordOf(message.data)
      const title = stringOf(record.title)
      if (title === '') {
        dismiss('window')
        return
      }
      show(
        entryOf({
          id: 'window',
          priority: PRIORITY_IDLE,
          label: title,
          detail: appNameOf(record),
        }),
        0,
      )
    }),
  )

  // The sink watcher republishes on every pipewire event, and starting a new
  // track is one, so only a level that actually moved surfaces the island.
  $effect(() =>
    subscribeTo(bus, 'system.volume', (message) => {
      const record = recordOf(message.data)
      const percent = record.volume
      if (typeof percent !== 'number') {
        return
      }
      const level = `${percent}:${record.muted === true}`
      if (level === lastVolumeLevel) {
        return
      }
      const isFirst = lastVolumeLevel === ''
      lastVolumeLevel = level
      if (isFirst) {
        return
      }
      show(volumeEntry(percent, record.muted === true), TRANSIENT_TTL_MS)
    }),
  )

  // A loaded track is the island's standing content: above the focused window
  // it displaces, below anything transient that interrupts it.
  $effect(() =>
    subscribeTo(bus, 'media.player', (message) => {
      const record = recordOf(message.data)
      const title = stringOf(record.title)
      const status = stringOf(record.status)
      if (title === '' || status === '' || status === 'Stopped') {
        dismiss(MEDIA_ID)
        return
      }
      // The playhead is polled, so a new track would otherwise show the
      // previous one's elapsed time until the next tick lands.
      if (title !== lastTrackTitle) {
        position = 0
        lastTrackTitle = title
      }
      media = {
        status,
        length: numberOf(record.length, 0),
        shuffle: record.shuffle === true,
      }
      show(
        entryOf({
          id: MEDIA_ID,
          priority: PRIORITY_ACTIVITY,
          label: title,
          detail: stringOf(record.artist),
          art: artSourceOf(stringOf(record.artUrl)),
        }),
        0,
      )
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'media.position', (message) => {
      position = numberOf(recordOf(message.data).position, 0)
    }),
  )

  $effect(() => () => {
    for (const timer of dismissTimers.values()) {
      clearTimeout(timer)
    }
    dismissTimers.clear()
  })

  // Opening is a decision about one entry. A transient alert displacing it, or
  // the entry expiring, ends that decision.
  $effect(() => {
    const id = idOf(current)
    if (id === openedId) {
      return
    }
    openedId = id
    opened = false
  })

  function dismiss(id: string): void {
    const timer = dismissTimers.get(id)
    if (timer !== undefined) {
      clearTimeout(timer)
      dismissTimers.delete(id)
    }
    entries = entries.filter((entry) => entry.id !== id)
  }

  function show(entry: IslandEntry, ttlMs: number): void {
    dismiss(entry.id)
    entries = [...entries.filter((e) => e.id !== entry.id), entry]
    if (ttlMs <= 0) {
      return
    }
    dismissTimers.set(
      entry.id,
      setTimeout(() => dismiss(entry.id), ttlMs),
    )
  }

  function seekTo(press: { x: number }, trackWidth: number): void {
    if (media.length <= 0 || trackWidth <= 0) {
      return
    }
    const fraction = Math.min(Math.max(press.x / trackWidth, 0), 1)
    void bus.call('media:seek', { seconds: fraction * media.length })
  }

  // The stack's winner is the highest priority, ties going to whatever arrived
  // last: a second notification supersedes the one before it.
  function topEntry(stack: IslandEntry[]): IslandEntry | null {
    let best: IslandEntry | null = null
    for (const entry of stack) {
      if (best === null || entry.priority >= best.priority) {
        best = entry
      }
    }
    return best
  }

  function modeOf(entry: IslandEntry | null, isOpened: boolean, mediaShowing: boolean): string {
    if (entry === null) {
      return 'empty'
    }
    if (isOpened && mediaShowing) {
      return 'expandedMedia'
    }
    if (isOpened) {
      return 'expanded'
    }
    if (entry.priority > PRIORITY_IDLE) {
      return 'active'
    }
    return 'rest'
  }

  // The open island is already at its full size; growing it further on hover
  // would read as a second, accidental expansion.
  function grownSize(base: Dimensions, currentMode: string, isHovered: boolean): Dimensions {
    if (!isHovered || currentMode.startsWith('expanded')) {
      return base
    }
    return {
      width: base.width + HOVER_GROWTH.width,
      height: base.height + HOVER_GROWTH.height,
    }
  }

  function entryOf(fields: Partial<IslandEntry> & { id: string }): IslandEntry {
    return {
      priority: PRIORITY_ACTIVITY,
      label: '',
      detail: '',
      progress: NO_PROGRESS,
      tone: 'neutral',
      art: '',
      ...fields,
    }
  }

  function entryFromMessage(data: unknown): { entry: IslandEntry; ttlMs: number } | null {
    const record = recordOf(data)
    const id = stringOf(record.id)
    if (id === '') {
      console.error('notch: island:show without an id, ignored')
      return null
    }
    return {
      entry: entryOf({
        id,
        priority: numberOf(record.priority, PRIORITY_ALERT),
        label: stringOf(record.label),
        detail: stringOf(record.detail),
        progress: numberOf(record.progress, NO_PROGRESS),
        tone: toneOf(record.tone),
      }),
      ttlMs: numberOf(record.ttlMs, 0),
    }
  }

  function volumeEntry(percent: number, muted: boolean): IslandEntry {
    if (muted) {
      return entryOf({
        id: 'volume',
        priority: PRIORITY_TRANSIENT,
        label: 'Muted',
        tone: 'accent',
      })
    }
    return entryOf({
      id: 'volume',
      priority: PRIORITY_TRANSIENT,
      label: 'Volume',
      detail: `${percent}%`,
      progress: percent / 100,
      tone: 'accent',
    })
  }

  // Local art is a path the image loads directly. Remote art would have to be
  // downloaded first and is skipped.
  function artSourceOf(artUrl: string): string {
    if (artUrl.startsWith('file://')) {
      return decodeURIComponent(artUrl.slice('file://'.length))
    }
    return ''
  }

  function playPauseIcon(state: MediaState): string {
    if (state.status === 'Playing') {
      return ICONS.pause
    }
    return ICONS.play
  }

  function appNameOf(record: Record<string, unknown>): string {
    const initialClass = stringOf(record.initialClass)
    if (initialClass !== '') {
      return initialClass
    }
    return stringOf(record.class)
  }

  function idOf(entry: IslandEntry | null): string {
    if (entry === null) {
      return ''
    }
    return entry.id
  }

  function stringOf(value: unknown): string {
    if (typeof value === 'string') {
      return value
    }
    return ''
  }

  function numberOf(value: unknown, fallback: number): number {
    if (typeof value === 'number') {
      return value
    }
    return fallback
  }

  function toneOf(value: unknown): string {
    if (typeof value === 'string' && TONE_COLORS[value] !== undefined) {
      return value
    }
    return 'neutral'
  }

  function toneColor(entry: IslandEntry): string {
    return TONE_COLORS[entry.tone]
  }

  function headerStyle(currentMode: string): Style {
    if (currentMode === 'expandedMedia') {
      return { paddingTop: 4, paddingX: 16, minHeight: 64 }
    }
    return { paddingX: 12, minHeight: 30 }
  }

  function artStyle(large: boolean): Style {
    if (large) {
      return { width: 48, height: 48, radius: 8 }
    }
    return { width: 20, height: 20, radius: 6 }
  }

  function titleStyle(large: boolean): Style {
    if (large) {
      return { fontSize: 15, fontWeight: 600 }
    }
    return { fontWeight: 500 }
  }

  function controlColor(active: boolean): string {
    if (active) {
      return colors.accent
    }
    return alpha(colors.content, 0.85)
  }

  function tooltipText(entry: IslandEntry | null): string {
    if (entry === null) {
      return ''
    }
    return entry.label
  }

  function progressWidth(entry: IslandEntry): Size {
    const clamped = Math.min(Math.max(entry.progress, 0), 1)
    return `${clamped * 100}%`
  }

  function playedWidth(state: MediaState, seconds: number): Size {
    if (state.length <= 0) {
      return '0%'
    }
    const fraction = Math.min(Math.max(seconds / state.length, 0), 1)
    return `${fraction * 100}%`
  }

  function remainingLabel(state: MediaState, seconds: number): string {
    if (state.length <= 0) {
      return ''
    }
    return `-${clockLabel(state.length - seconds)}`
  }

  function clockLabel(seconds: number): string {
    const whole = Math.max(Math.floor(seconds), 0)
    const minutes = Math.floor(whole / 60)
    return `${minutes}:${String(whole % 60).padStart(2, '0')}`
  }

  const scrubTrackWidth = SIZES.expandedMedia.width - 2 * ISLAND_PADDING - 2 * (SCRUB_TIME_WIDTH + 10)
</script>

<Surface
  {args}
  width={WINDOW_WIDTH}
  height={WINDOW_HEIGHT}
  style={{ direction: 'row', justify: 'center', align: 'start' }}
>
  <Fillet side="left" />

  <Tooltip
    text={tooltipText(current)}
    side="bottom"
    offset={TOOLTIP_OFFSET}
    disabled={isExpanded || current === null}
  >

  <!-- The content keeps its natural size at the top edge throughout and the
       growing island wipes it into view. -->
    <View
      input
      style={{
        width: islandWidth,
        height: islandHeight,
        overflow: 'hidden',
        background: colors.canvas,
        color: colors.content,
        fontSize: 12,
        radius: [0, 0, ISLAND_RADIUS, ISLAND_RADIUS],
      }}
      onhover={(isHovered) => (hovered = isHovered)}
    >
      {#if current !== null}
        <Button
          style={{ ...headerStyle(mode), align: 'center', color: colors.content }}
          onpress={() => (opened = !opened)}
        >
          {#if isMedia}
            {@render mediaHeader(current, mode === 'expandedMedia')}
          {:else}
            {@render plainHeader(current)}
          {/if}
        </Button>
      {/if}

      {#if mode === 'expandedMedia'}
        {@render scrubber()}
      {/if}
      {#if mode === 'expanded' && current !== null}
        {@render expandedDetail(current)}
      {/if}

      {#if current !== null && current.progress >= 0 && !isExpanded}
        <View
          style={{
            position: 'absolute',
            left: 16,
            right: 16,
            bottom: 6,
            height: 3,
            radius: 2,
            background: alpha(colors.content, 0.15),
          }}
        >
          <View
            style={{
              width: progressWidth(current),
              height: 3,
              radius: 2,
              background: toneColor(current),
              transition: 200,
            }}
          ></View>
        </View>
      {/if}
    </View>
  </Tooltip>

  <Fillet side="right" />
</Surface>

{#snippet plainHeader(entry: IslandEntry)}
  <View style={{ flex: 1, minWidth: 0, direction: 'row', align: 'center', gap: 10 }}>
    <View style={{ width: 6, height: 6, radius: 3, background: toneColor(entry) }}></View>
    <Text style={{ flex: 1, minWidth: 0, fontWeight: 500, truncate: true }}>{entry.label}</Text>
    {#if entry.detail !== '' && !isExpanded}
      <Text style={{ color: alpha(colors.content, 0.5) }}>{entry.detail}</Text>
    {/if}
  </View>
{/snippet}

<!-- Collapsed and open share this row: sleeve on the left, title in the
     middle, level meter on the right. Only the scale changes. -->
{#snippet mediaHeader(entry: IslandEntry, large: boolean)}
  <View style={{ flex: 1, minWidth: 0, direction: 'row', align: 'center', gap: 12 }}>
    <View style={{ ...artStyle(large), background: alpha(colors.content, 0.1), overflow: 'hidden' }}>
      {#if entry.art !== ''}
        <Image src={entry.art} fit="cover" style={{ width: '100%', height: '100%' }} />
      {/if}
    </View>
    <View style={{ flex: 1, minWidth: 0, gap: 4, justify: 'center' }}>
      <Text style={{ ...titleStyle(large), truncate: true }}>{entry.label}</Text>
      {#if large && entry.detail !== ''}
        <Text style={{ fontSize: 13, color: alpha(colors.content, 0.6), truncate: true }}>{entry.detail}</Text>
      {/if}
    </View>
    <LevelMeter {large} playing={media.status === 'Playing'} />
  </View>
{/snippet}

{#snippet expandedDetail(entry: IslandEntry)}
  <View style={{ gap: 4, paddingX: ISLAND_PADDING, paddingBottom: ISLAND_PADDING }}>
    {#if entry.detail !== ''}
      <Text style={{ fontSize: 13, color: alpha(colors.content, 0.7) }}>{entry.detail}</Text>
    {/if}
    <Text style={{ fontSize: 10, color: alpha(colors.content, 0.3) }}>{entry.id.toUpperCase()}</Text>
  </View>
{/snippet}

{#snippet scrubber()}
  <View style={{ gap: 12, paddingX: ISLAND_PADDING, paddingBottom: ISLAND_PADDING }}>
    <View style={{ direction: 'row', align: 'center', gap: 10 }}>
      <Text style={{ width: SCRUB_TIME_WIDTH, fontSize: 11, color: alpha(colors.content, 0.5) }}>
        {clockLabel(position)}
      </Text>
      <View
        style={{ flex: 1, height: 6, radius: 3, background: alpha(colors.content, 0.15), overflow: 'hidden' }}
        onpress={(event) => seekTo(event, scrubTrackWidth)}
      >
        <View
          style={{
            width: playedWidth(media, position),
            height: 6,
            radius: 3,
            background: alpha(colors.content, 0.7),
          }}
        ></View>
      </View>
      <Text style={{ width: SCRUB_TIME_WIDTH, fontSize: 11, color: alpha(colors.content, 0.5) }}>
        {remainingLabel(media, position)}
      </Text>
    </View>
    {@render transport()}
  </View>
{/snippet}

<!-- The transport group sits on the centre line whatever the shuffle button's
     width: the shuffle button is placed over the row's start instead of being
     one of two columns. -->
{#snippet transport()}
  <View style={{ direction: 'row', align: 'center', justify: 'center' }}>
    <View style={{ position: 'absolute', left: 0 }}>
      {@render control(ICONS.shuffle, 20, media.shuffle, 'media:shuffle')}
    </View>
    <View style={{ direction: 'row', gap: 24, align: 'center' }}>
      {@render control(ICONS.previous, 22, false, 'media:previous')}
      {@render control(playPauseIcon(media), 26, false, 'media:playPause')}
      {@render control(ICONS.next, 22, false, 'media:next')}
    </View>
  </View>
{/snippet}

{#snippet control(icon: string, pixels: number, active: boolean, action: string)}
  <Button
    style={{ padding: 4, radius: 8, color: controlColor(active), transition: 120 }}
    hover={{ background: alpha(colors.content, 0.1) }}
    onpress={() => void bus.call(action, {})}
  >
    <Icon name={icon} size={pixels} />
  </Button>
{/snippet}
