<script lang="ts">
  import { Tween } from 'svelte/motion'
  import { Button, Icon, Image, Popover, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { PointerEvent } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import { cubicBezier } from '../../lib/easing'
  import type { ViewProps } from '../../host/plugins/views'
  import { recordOf } from '../../lib/record'
  import Surface from '../../Surface.svelte'
  import { readTextFile } from '../../system/fs'
  import { alpha, colors, windowBackground } from '../../theme'
  import DockIcon from './DockIcon.svelte'
  import {
    configuredAppsOf,
    dockAppOf,
    dockEntries,
    focusClass,
    magnificationOf,
    openClassesOf,
    openWindowsOf,
    pinnedNamesOf,
    resolvePinned,
    windowsOfApp,
    workspaceLabel,
  } from './dockModel'
  import type { DockApp, OpenWindow } from './dockModel'

  // The dock: a centred floating panel of launchers, hidden below the screen
  // edge until the pointer enters the hot strip along it.
  //
  // The surface is always tall enough for the window previews, which rise
  // above the panel inside it. Only the hot strip, the revealed panel and an
  // open preview take the pointer; the rest of the surface is click-through.
  //
  //   dock:reveal  {open?}  raise the dock, or lower it with open: false
  //
  // Entries are the pinned apps followed by every app with an open window
  // (hypr.windows) that is not already pinned. Pins come from the config key
  // dock.pinned (ids or names matched against the desktop-entry catalog), or
  // from dock.apps, which carries whole records. Clicking a running app focuses
  // its window instead of launching a second instance; with several windows
  // open it raises a panel of window previews to pick from. A right click
  // raises the panel for any running app, one window or many.

  interface Preview {
    image: string
  }

  const LAUNCH_TIMEOUT_MS = 15000
  const ICON_PIXELS = 44
  const TILE_WIDTH = 52
  const HOT_STRIP_PIXELS = 4
  const DEFAULT_HEIGHT = 96
  const SLIDE_MS = 200
  const SLIDE_DISTANCE = 100
  const PREVIEW_WIDTH = 280
  const PREVIEW_HEIGHT = 170
  const PREVIEW_AREA = 250
  const PREVIEW_GAP = 8
  // How long the pointer rests on a tile before its windows are captured, so
  // a sweep across the dock does not capture every running app on the way.
  const PRECAPTURE_DELAY_MS = 120
  // Long enough to cross onto another output and back without the dock going.
  const COLLAPSE_DELAY_MS = 400

  let { bus, args }: ViewProps = $props()

  let catalog = $state<DockApp[]>([])
  let openWindows = $state<OpenWindow[]>([])
  let pinnedNames = $state<string[]>([])
  let configuredApps = $state<DockApp[]>([])
  let revealed = $state(false)
  let hoveredIndex = $state(-1)
  let launching = $state<ReadonlySet<string>>(new Set())
  // Which tile's preview panel is up, and the captured frame per window
  // address; a window without a capture shows the app icon instead. Frames
  // are captured while the pointer rests on a tile, so the panel opens with
  // them already in hand, and refreshed once more when it opens.
  let previewIndex = $state(-1)
  let previews = $state<Record<string, Preview>>({})
  let precaptureTimer: ReturnType<typeof setTimeout> | null = null

  // Either half of the hot area keeps the dock open, so travelling from the
  // strip into the panel never passes through a moment where neither is under
  // the pointer. Leaving is delayed on top of that: the pointer crossing onto
  // another output leaves the surface outright, and a dock that vanished the
  // instant it did would be unusable on a multi-monitor desk.
  let overStrip = false
  let overPanel = false
  let collapseTimer: ReturnType<typeof setTimeout> | null = null
  const processNames = new Map<number, string>()

  const slide = new Tween(0, { duration: SLIDE_MS, easing: cubicBezier(0.25, 0.1, 0.25, 1) })

  const dockHeight = $derived(heightArgOf(args))
  const pinned = $derived(resolvePinned(catalog, pinnedNames, configuredApps))
  const openClasses = $derived(openClassesOf(openWindows))
  const entries = $derived(dockEntries(catalog, pinned, openWindows))
  const openKeys = $derived(new Set(openClasses.map((wmClass) => wmClass.toLowerCase())))
  const expanded = $derived(revealed || slide.current > 0.001)
  const windowHeight = $derived(dockHeight + PREVIEW_AREA)

  $effect(() => {
    void slide.set(Number(revealed))
  })

  $effect(() =>
    subscribeTo(bus, 'hypr.windows', (message) => {
      const windows = openWindowsOf(message.data)
      openWindows = windows
      void nameProcesses(windows)
      stopLandedLaunches()
      dropClosedPreviews()
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'config', (message) => {
      pinnedNames = pinnedNamesOf(message.data)
      configuredApps = configuredAppsOf(message.data)
    }),
  )

  // A keybind or a script can raise and lower the dock without the pointer.
  $effect(() =>
    subscribeTo(bus, 'dock:reveal', (message) => {
      revealed = recordOf(message.data).open !== false
    }),
  )

  $effect(() => {
    void loadCatalog()
  })

  function heightArgOf(value: unknown): number {
    const height = recordOf(value).height
    if (typeof height === 'number' && height > 0) {
      return height
    }
    return DEFAULT_HEIGHT
  }

  // The apps extension may not have mounted yet, in which case the call times
  // out: the dock stays empty rather than the rejection escaping into a
  // warning with no context.
  async function loadCatalog(): Promise<void> {
    let reply: unknown
    try {
      reply = await bus.call('apps:list', {})
    } catch (error) {
      console.error('dock: cannot read the application catalog:', error)
      return
    }
    if (!Array.isArray(reply)) {
      return
    }
    catalog = reply.map(dockAppOf)
  }

  // The process name behind each window comes from /proc, one read per
  // process, and fills in once it has arrived.
  async function nameProcesses(windows: OpenWindow[]): Promise<void> {
    for (const window of windows) {
      if (window.pid > 0 && !processNames.has(window.pid)) {
        processNames.set(window.pid, await processNameOf(window.pid))
      }
    }
    openWindows = openWindows.map((window) => ({ ...window, command: commandOf(window.pid) }))
  }

  function commandOf(pid: number): string {
    const name = processNames.get(pid)
    if (name === undefined) {
      return ''
    }
    return name
  }

  async function processNameOf(pid: number): Promise<string> {
    const comm = await readTextFile(`/proc/${pid}/comm`)
    if (comm === null) {
      return ''
    }
    return comm.trim()
  }

  function isOpen(app: DockApp): boolean {
    if (app.wmClass !== '') {
      return openKeys.has(app.wmClass.toLowerCase())
    }
    return openKeys.has(app.id.toLowerCase())
  }

  async function activate(app: DockApp, index: number, secondary: boolean): Promise<void> {
    const windows = windowsOfApp(openWindows, app)
    if (windows.length === 0) {
      await launch(app)
      return
    }
    if (windows.length === 1 && !secondary) {
      focusWindow(windows[0])
      return
    }
    await openPreviews(index, windows)
  }

  async function launch(app: DockApp): Promise<void> {
    startLaunch(app.id)
    const reply = recordOf(await bus.call('apps:launch', { command: app.exec }))
    if (reply.error === undefined) {
      return
    }
    console.error(`dock: launching ${app.name} failed:`, reply.error)
    stopLaunch(app.id)
  }

  // The icon hops until the app's window turns up on hypr.windows; the timeout
  // ends it for apps that never map one (or that died on startup).
  function startLaunch(id: string): void {
    launching = new Set(launching).add(id)
    setTimeout(() => stopLaunch(id), LAUNCH_TIMEOUT_MS)
  }

  function stopLaunch(id: string): void {
    if (!launching.has(id)) {
      return
    }
    const next = new Set(launching)
    next.delete(id)
    launching = next
  }

  function stopLandedLaunches(): void {
    for (const app of entries) {
      if (launching.has(app.id) && isOpen(app)) {
        stopLaunch(app.id)
      }
    }
  }

  // A window is addressed by its Hyprland handle: a class match is a
  // case-sensitive regex there, and a desktop entry's StartupWMClass rarely
  // spells the class the way the window reports it.
  function focusWindow(window: OpenWindow): void {
    previewIndex = -1
    void bus.call('hypr:dispatch', { dispatcher: 'focuswindow', arg: `address:${window.address}` })
  }

  // With every frame already captured on hover the panel opens at once and
  // the frames refresh behind it; otherwise it waits for the first capture so
  // it opens complete rather than filling in tile by tile. A failed capture
  // leaves that entry on the app icon.
  async function openPreviews(index: number, windows: OpenWindow[]): Promise<void> {
    cancelPrecapture()
    if (windows.every((window) => previews[window.address] !== undefined)) {
      previewIndex = index
      void captureAll(windows)
      return
    }
    await captureAll(windows)
    previewIndex = index
  }

  async function captureAll(windows: OpenWindow[]): Promise<void> {
    await Promise.all(
      windows.map(async (window) => {
        const preview = await captureWindow(window)
        if (preview !== null) {
          previews = { ...previews, [window.address]: preview }
        }
      }),
    )
  }

  function schedulePrecapture(app: DockApp): void {
    cancelPrecapture()
    const windows = windowsOfApp(openWindows, app)
    if (windows.length === 0) {
      return
    }
    precaptureTimer = setTimeout(() => {
      precaptureTimer = null
      void captureAll(windows)
    }, PRECAPTURE_DELAY_MS)
  }

  function cancelPrecapture(): void {
    if (precaptureTimer === null) {
      return
    }
    clearTimeout(precaptureTimer)
    precaptureTimer = null
  }

  // Frames of windows that have gone are dropped, so a closed window's
  // frame is not held for the rest of the session.
  function dropClosedPreviews(): void {
    const open = new Set(openWindows.map((window) => window.address))
    const kept: Record<string, Preview> = {}
    for (const [address, preview] of Object.entries(previews)) {
      if (open.has(address)) {
        kept[address] = preview
      }
    }
    previews = kept
  }

  async function captureWindow(window: OpenWindow): Promise<Preview | null> {
    const reply = recordOf(
      await bus.call('hypr:capture', {
        address: window.address,
        width: PREVIEW_WIDTH,
        height: PREVIEW_HEIGHT,
      }),
    )
    if (typeof reply.image === 'string') {
      return { image: reply.image }
    }
    console.warn(`dock: no preview for ${window.title}:`, reply.error)
    return null
  }

  function dismissPreviews(index: number): void {
    if (previewIndex !== index) {
      return
    }
    previewIndex = -1
    if (!overStrip && !overPanel) {
      scheduleCollapse()
    }
  }

  function hoverPanel(isOver: boolean): void {
    if (isOver) {
      enter('panel')
      return
    }
    leave('panel')
  }

  function hoverStrip(isOver: boolean): void {
    if (isOver) {
      enter('strip')
      return
    }
    leave('strip')
  }

  function hoverTile(index: number, isOver: boolean): void {
    if (isOver) {
      hover(index)
      return
    }
    unhover(index)
  }

  function hover(index: number): void {
    hoveredIndex = index
    schedulePrecapture(entries[index])
  }

  function unhover(index: number): void {
    cancelPrecapture()
    if (hoveredIndex === index) {
      hoveredIndex = -1
    }
  }

  function enter(part: 'strip' | 'panel'): void {
    setOver(part, true)
    cancelCollapse()
    revealed = true
  }

  function leave(part: 'strip' | 'panel'): void {
    setOver(part, false)
    if (part === 'panel') {
      hoveredIndex = -1
    }
    scheduleCollapse()
  }

  function setOver(part: 'strip' | 'panel', value: boolean): void {
    if (part === 'strip') {
      overStrip = value
      return
    }
    overPanel = value
  }

  function scheduleCollapse(): void {
    cancelCollapse()
    collapseTimer = setTimeout(() => {
      collapseTimer = null
      if (overStrip || overPanel || previewIndex >= 0) {
        return
      }
      revealed = false
      hoveredIndex = -1
    }, COLLAPSE_DELAY_MS)
  }

  function cancelCollapse(): void {
    if (collapseTimer === null) {
      return
    }
    clearTimeout(collapseTimer)
    collapseTimer = null
  }

  function isHopping(app: DockApp): boolean {
    return launching.has(app.id)
  }

  function dotColor(app: DockApp): string {
    if (isOpen(app)) {
      return alpha(colors.content, 0.8)
    }
    return 'transparent'
  }

  function onTilePress(app: DockApp, index: number, event: PointerEvent): void {
    void activate(app, index, event.button === 2)
  }

  $effect(() => cancelCollapse)
  $effect(() => cancelPrecapture)
</script>

<Surface
  {args}
  height={windowHeight}
  style={{ background: windowBackground, justify: 'end' }}
>
  {#if previewIndex >= 0}
    <!-- Catches the press that puts the previews away, which would otherwise
         reach whatever is below the surface. -->
    <View
      input
      style={{ position: 'absolute', inset: 0 }}
      ondown={() => dismissPreviews(previewIndex)}
    ></View>
  {/if}

  {#if expanded}
    <View
      input
      style={{
        alignSelf: 'center',
        paddingBottom: 8,
        translate: [0, (1 - slide.current) * SLIDE_DISTANCE],
      }}
      onhover={hoverPanel}
    >
      <View
        style={{
          direction: 'row',
          gap: 4,
          paddingX: 8,
          paddingY: 6,
          background: alpha(colors.elevated, 0.4),
          borderWidth: 1,
          borderColor: alpha(colors.content, 0.1),
          radius: 16,
        }}
      >
        {#each entries as app, index (app.id)}
          {@render tile(app, index)}
        {/each}
      </View>
    </View>
  {/if}

  <!-- The hot strip is the only part of a hidden dock the pointer reaches. It
       has to paint, or a surface that draws nothing is never mapped. -->
  <View
    input
    style={{ height: HOT_STRIP_PIXELS, background: windowBackground }}
    onhover={hoverStrip}
  ></View>
</Surface>

{#snippet tile(app: DockApp, index: number)}
  {@const magnification = magnificationOf(index, hoveredIndex)}
  {@const windows = windowsOfApp(openWindows, app)}
  <Popover
    open={previewIndex === index}
    side="top"
    align="center"
    input
    focusPanel={false}
    style={{
      padding: 4,
      background: alpha(colors.elevated, 0.4),
      borderWidth: 1,
      borderColor: alpha(colors.content, 0.1),
      radius: 16,
    }}
    onclose={() => dismissPreviews(index)}
  >
    {#snippet trigger()}
      <Tooltip text={app.name} side="top" disabled={previewIndex >= 0}>
        <View
          style={{ width: TILE_WIDTH, paddingBottom: 6, align: 'center' }}
          onhover={(isOver) => hoverTile(index, isOver)}
          onpress={(event) => onTilePress(app, index, event)}
          buttons={['right']}
          ondown={(event) => onTilePress(app, index, event)}
        >
          <View style={{ width: ICON_PIXELS, height: ICON_PIXELS }}>
            <DockIcon icon={app.icon} size={ICON_PIXELS} {magnification} hopping={isHopping(app)} />
          </View>
          <View style={{ width: 4, height: 4, radius: 2, marginTop: 0, background: dotColor(app) }}></View>
        </View>
      </Tooltip>
    {/snippet}
    <View style={{ direction: 'row', gap: PREVIEW_GAP }} onhover={hoverPanel}>
      {#each windows as window (window.address)}
        {@render previewCard(window, app)}
      {/each}
    </View>
  </Popover>
{/snippet}

{#snippet previewCard(window: OpenWindow, app: DockApp)}
  <Button
    style={{ padding: 6, radius: 12, color: colors.content }}
    hover={{ background: alpha(colors.content, 0.08) }}
    onpress={() => focusWindow(window)}
  >
    <View style={{ width: PREVIEW_WIDTH, gap: 4 }}>
      <View
        style={{
          width: PREVIEW_WIDTH,
          height: PREVIEW_HEIGHT,
          radius: 8,
          overflow: 'hidden',
          background: alpha('#000000', 0.3),
          align: 'center',
          justify: 'center',
        }}
      >
        {#if previews[window.address] !== undefined}
          <Image src={previews[window.address].image} fit="contain" style={{ width: PREVIEW_WIDTH, height: PREVIEW_HEIGHT }} />
        {:else}
          <Icon name={app.icon} fallback="application-x-executable" size={48} />
        {/if}
      </View>
      <Text style={{ fontSize: 12, truncate: true }}>{window.title}</Text>
      <Text style={{ fontSize: 11, color: alpha(colors.content, 0.6) }}>{workspaceLabel(window)}</Text>
    </View>
  </Button>
{/snippet}
