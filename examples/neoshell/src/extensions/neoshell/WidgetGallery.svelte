<script lang="ts">
  import { Button, Input, Presence, Scroll, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import { fly } from 'svelte/transition'
  import { subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { arrayOf, numberOf, recordOf, stringOf } from '../../lib/record'
  import Surface from '../../Surface.svelte'
  import { surfaceOf } from '../../surface'
  import { alpha, colors, windowBackground } from '../../theme'
  import WidgetPreview from './WidgetPreview.svelte'

  // The widget gallery: a panel that rises from the bottom of the screen and
  // lists every widget the installed extensions declare, grouped by the
  // extension that provides them.
  //
  // A widget is added by clicking its tile; the desktop's layout places it.
  // Two layer surfaces do not share a pointer grab, so a drag cannot cross
  // from this window to the desktop's.
  //
  // The window spans the output's width so the panel can centre itself in it.
  // While closed the window draws a zero-size input marker, which is what
  // leaves the bottom of the screen click-through.

  const PREVIEW_WIDTH_PX = 176
  // Every preview is the small size. A widget is resized once it is on the
  // desktop, from its own context menu, so the gallery has one thing to show
  // and one thing to add.
  const PREVIEW_SIZE = 'small'
  const ALL = 'All Widgets'
  const WIDTH_FRACTION = 0.6
  const HEIGHT_FRACTION = 0.46
  const FALLBACK_SIZE = { width: 1920, height: 1080 }
  const SIDEBAR_WIDTH = 224
  const RISE_MS = 250

  interface CatalogEntry {
    type: string
    name: string
    category: string
    description: string
  }

  let { bus, registry, args }: ViewProps = $props()

  const surface = surfaceOf()

  let open = $state(false)
  let catalog = $state<CatalogEntry[]>([])
  let category = $state(ALL)
  let query = $state('')
  let viewport = $state(FALLBACK_SIZE)
  // The registry is outside Svelte's reactivity, so a widget provider that
  // registers after the gallery is up is picked up by bumping this.
  let generation = $state(0)

  const categories = $derived([ALL, ...distinctCategories(catalog)])
  const shown = $derived(filtered(catalog, category, query))
  const panelWidth = $derived(Math.round(viewport.width * WIDTH_FRACTION))
  const panelHeight = $derived(Math.round(viewport.height * HEIGHT_FRACTION))

  $effect(() =>
    subscribeTo(bus, 'widgets:gallery', (message) => {
      open = recordOf(message.data).open === true
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'widgets.catalog', (message) => {
      catalog = entriesOf(recordOf(message.data).widgets)
    }),
  )

  // The panel is a fraction of the output, and the window is as tall as the
  // panel — so its size comes from the compositor bridge.
  $effect(() =>
    subscribeTo(bus, 'hypr.monitors', (message) => {
      applyMonitors(message.data)
    }),
  )

  $effect(() =>
    registry.onChange(() => {
      generation += 1
    }),
  )

  function applyMonitors(data: unknown): void {
    const monitor = ownMonitor(arrayOf(data))
    if (monitor === undefined) {
      return
    }
    const scale = Math.max(numberOf(monitor.scale, 1), 0.1)
    viewport = {
      width: numberOf(monitor.width, FALLBACK_SIZE.width) / scale,
      height: numberOf(monitor.height, FALLBACK_SIZE.height) / scale,
    }
  }

  function ownMonitor(monitors: Record<string, unknown>[]): Record<string, unknown> | undefined {
    const own = monitors.find((monitor) => monitor.name === surface.monitor)
    if (own !== undefined) {
      return own
    }
    return monitors[0]
  }

  function close(): void {
    open = false
    bus.publish('widgets:gallery', { open: false })
  }

  function add(entry: CatalogEntry): void {
    bus.publish('widgets:add', { type: entry.type, size: PREVIEW_SIZE })
    close()
  }

  function distinctCategories(entries: CatalogEntry[]): string[] {
    return [...new Set(entries.map((entry) => entry.category))].sort()
  }

  function filtered(entries: CatalogEntry[], active: string, search: string): CatalogEntry[] {
    const needle = search.trim().toLowerCase()
    return entries.filter((entry) => inCategory(entry, active) && matches(entry, needle))
  }

  function inCategory(entry: CatalogEntry, active: string): boolean {
    if (active === ALL) {
      return true
    }
    return entry.category === active
  }

  function matches(entry: CatalogEntry, needle: string): boolean {
    if (needle === '') {
      return true
    }
    return `${entry.name} ${entry.category} ${entry.description}`.toLowerCase().includes(needle)
  }

  function entriesOf(value: unknown): CatalogEntry[] {
    return arrayOf(value).flatMap(entryOf)
  }

  function entryOf(raw: Record<string, unknown>): CatalogEntry[] {
    if (typeof raw.type !== 'string' || typeof raw.name !== 'string') {
      return []
    }
    return [
      {
        type: raw.type,
        name: raw.name,
        category: stringOr(raw.category, 'Other'),
        description: stringOf(raw.description),
      },
    ]
  }

  function stringOr(value: unknown, fallback: string): string {
    if (typeof value !== 'string' || value === '') {
      return fallback
    }
    return value
  }

  function categoryBackground(name: string, active: string): string {
    if (name === active) {
      return alpha(colors.content, 0.15)
    }
    return 'transparent'
  }

  function categoryWeight(name: string, active: string): number {
    if (name === active) {
      return 500
    }
    return 400
  }
</script>

<Surface
  {args}
  height={panelHeight}
  keyboard="on-demand"
  style={{ background: windowBackground, justify: 'end', align: 'center' }}
>
  {#if open}
    <Presence
      in={[fly, { y: panelHeight, duration: RISE_MS }]}
      out={[fly, { y: panelHeight, duration: RISE_MS }]}
      style={{ width: panelWidth, height: panelHeight }}
    >
      <View
        input
        style={{
          flex: 1,
          direction: 'row',
          overflow: 'hidden',
          background: alpha(colors.surface, 0.8),
          borderWidth: 1,
          borderColor: alpha(colors.content, 0.1),
          radius: [24, 24, 0, 0],
          shadow: { y: -10, blur: 40, color: alpha('#000000', 0.5) },
        }}
      >
        {@render sidebar()}
        {@render body()}
      </View>
    </Presence>
  {:else}
    <View input style={{ width: 0, height: 0 }}></View>
  {/if}
</Surface>

{#snippet sidebar()}
  <View
    style={{
      width: SIDEBAR_WIDTH,
      gap: 12,
      padding: 16,
      borderRightWidth: 1,
      borderColor: alpha(colors.content, 0.1),
    }}
  >
    <Input
      bind:value={query}
      placeholder="Search Widgets"
      style={{
        fontSize: 13,
        paddingX: 10,
        paddingY: 6,
        radius: 8,
        background: alpha('#000000', 0.6),
        borderWidth: 1,
        borderColor: alpha(colors.content, 0.15),
      }}
      focus={{ borderColor: alpha(colors.content, 0.4) }}
    />

    <Scroll style={{ flex: 1 }}>
      <View style={{ gap: 2 }}>
        {#each categories as name (name)}
          <Button
            style={{
              paddingX: 12,
              paddingY: 8,
              radius: 8,
              fontSize: 13,
              fontWeight: categoryWeight(name, category),
              background: categoryBackground(name, category),
            }}
            hover={{ background: alpha(colors.content, 0.1) }}
            onpress={() => (category = name)}
          >
            <Text style={{ truncate: true }}>{name}</Text>
          </Button>
        {/each}
      </View>
    </Scroll>

    <Button
      style={{ paddingX: 12, paddingY: 6, radius: 8, fontSize: 13, background: colors.accent, color: colors.surface, justify: 'center' }}
      onpress={close}
    >
      <Text>Done</Text>
    </Button>
  </View>
{/snippet}

{#snippet body()}
  <Scroll style={{ flex: 1 }}>
    {#if shown.length === 0}
      <Text style={{ fontSize: 13, padding: 24, color: alpha(colors.content, 0.6) }}>No widgets match.</Text>
    {:else}
      <View style={{ direction: 'row', wrap: true, gap: 24, padding: 24, align: 'start' }}>
        {#each shown as entry (entry.type)}
          {@render tile(entry)}
        {/each}
      </View>
    {/if}
  </Scroll>
{/snippet}

{#snippet tile(entry: CatalogEntry)}
  <Tooltip text={entry.description} side="top" disabled={entry.description === ''}>
    <Button
      style={{ width: PREVIEW_WIDTH_PX, gap: 8, radius: 16 }}
      hover={{ background: alpha(colors.content, 0.08) }}
      onpress={() => add(entry)}
    >
      <View style={{ gap: 8, width: PREVIEW_WIDTH_PX }}>
        <WidgetPreview
          {bus}
          {registry}
          {generation}
          type={entry.type}
          size={PREVIEW_SIZE}
          width={PREVIEW_WIDTH_PX}
        />
        <Text style={{ fontSize: 14, fontWeight: 500, truncate: true }}>{entry.name}</Text>
        <Text style={{ fontSize: 12, color: alpha(colors.content, 0.6), truncate: true }}>{entry.category}</Text>
      </View>
    </Button>
  </Tooltip>
{/snippet}
