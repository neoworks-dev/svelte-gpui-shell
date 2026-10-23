<script lang="ts">
  import { tick } from 'svelte'
  import { Icon, Image, Input, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { PointerEvent } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import type { BusService } from '../../lib/bus'
  import { arrayOf, recordOf, stringOf } from '../../lib/record'
  import { alpha, colors } from '../../theme'
  import { rectOf } from './freeform'
  import type { Point, Rect } from './freeform'
  import type { DesktopEntry, DesktopStore, MenuItem } from './desktopStore.svelte'
  import { Glide } from './glide.svelte'
  import {
    extendsSelection,
    hasTravelled,
    PRIMARY_BUTTON,
    SECONDARY_BUTTON,
    travelOf,
    windowPointOf,
  } from './gestures'

  // The desktop folder's contents, placed free-form. A drag behaves the way a
  // Finder desktop does: a press selects at once, the selection is picked up
  // as translucent copies that follow the pointer exactly, the originals stay
  // where they are, and only the drop is snapped — to the screen edges and to
  // whatever is already there — with the icons gliding from where they were
  // let go to where they settle.
  //
  // Icon art comes from the icon theme by freedesktop name; an image file shows
  // itself.

  const CALL_TIMEOUT_MS = 10000
  const ICON_PIXELS = 48

  interface Drag {
    start: Point
    dx: number
    dy: number
    moved: boolean
    paths: string[]
    // The icon the pointer actually holds. It is the one whose landing is
    // snapped; the rest of the selection follows by the same delta so the
    // shape is kept.
    leadPath: string
    // The press did not change the selection, so a release without travel is
    // what narrows it to this icon (or toggles it, with ctrl).
    deferredSelect: boolean
    extend: boolean
  }

  interface Handler {
    id: string
    name: string
    command: string
  }

  let { bus, store }: { bus: BusService; store: DesktopStore } = $props()

  let drag = $state<Drag | null>(null)
  let settlePaths = $state<ReadonlySet<string>>(new Set())
  let renameDraft = $state('')
  let renameInput: Input | undefined = $state()
  const glide = new Glide()

  $effect(() =>
    subscribeTo(bus, 'files.desktop', (message) => {
      store.applyDesktopFolder(message.data)
    }),
  )

  const ghosts = $derived(ghostEntries(drag))

  function ghostEntries(current: Drag | null): DesktopEntry[] {
    if (current === null || !current.moved) {
      return []
    }
    return store.sortedEntries.filter((entry) => current.paths.includes(entry.path))
  }

  function pointOf(entry: DesktopEntry): Point {
    const point = store.iconPoint(entry.path)
    if (point === undefined) {
      return { x: 0, y: 0 }
    }
    return point
  }

  // An icon that has just been dropped is drawn short of its point until the
  // glide has carried it there.
  function shownPointOf(entry: DesktopEntry): Point {
    const point = pointOf(entry)
    if (!glide.active || !settlePaths.has(entry.path)) {
      return point
    }
    return { x: point.x + glide.offset.x, y: point.y + glide.offset.y }
  }

  function ghostPointOf(entry: DesktopEntry): Point {
    const point = pointOf(entry)
    if (drag === null) {
      return point
    }
    return { x: point.x + drag.dx, y: point.y + drag.dy }
  }

  function open(entry: DesktopEntry): void {
    void bus.call('files:open', { path: entry.path }, CALL_TIMEOUT_MS)
  }

  function openSelection(): void {
    for (const path of store.selection) {
      void bus.call('files:open', { path }, CALL_TIMEOUT_MS)
    }
  }

  function handleDown(entry: DesktopEntry, event: PointerEvent): void {
    if (event.button === SECONDARY_BUTTON) {
      void openMenu(entry, event)
      return
    }
    if (event.button !== PRIMARY_BUTTON) {
      return
    }
    if (event.clicks >= 2) {
      open(entry)
      return
    }
    pressIcon(entry, event)
  }

  // The press picks the icon up: an unselected one becomes the selection, a
  // selected one keeps the selection it is part of so the drag moves it all.
  function pressIcon(entry: DesktopEntry, event: PointerEvent): void {
    const wasSelected = store.isSelected(entry.path)
    if (!wasSelected) {
      pickUp(entry, extendsSelection(event))
    }
    glide.stop()
    drag = {
      start: windowPointOf(event),
      dx: 0,
      dy: 0,
      moved: false,
      paths: [...store.selection],
      leadPath: entry.path,
      deferredSelect: wasSelected,
      extend: extendsSelection(event),
    }
  }

  function pickUp(entry: DesktopEntry, extend: boolean): void {
    if (extend) {
      store.toggleSelected(entry.path)
      return
    }
    store.selectOnly(entry.path)
  }

  function handleMove(event: PointerEvent): void {
    if (drag === null) {
      return
    }
    const travel = travelOf(drag.start, event)
    const moved = drag.moved || (!store.desktopLocked && hasTravelled(travel))
    drag = { ...drag, dx: travel.x, dy: travel.y, moved }
  }

  function handleUp(entry: DesktopEntry): void {
    if (drag === null) {
      return
    }
    const finished = drag
    drag = null
    if (finished.moved) {
      dropDrag(finished)
      return
    }
    if (finished.deferredSelect) {
      narrowSelection(entry, finished.extend)
    }
  }

  function narrowSelection(entry: DesktopEntry, extend: boolean): void {
    if (extend) {
      store.toggleSelected(entry.path)
      return
    }
    store.selectOnly(entry.path)
  }

  // The lead icon's landing is snapped, and the delta that snap settled on is
  // what the whole selection moves by, so the selection keeps its shape. A
  // landing on something taken is refused and the originals never moved.
  function dropDrag(finished: Drag): void {
    const loose = { x: finished.dx, y: finished.dy }
    const delta = settledDelta(finished, loose)
    const targets = new Map<string, Point>()
    for (const path of finished.paths) {
      addTarget(targets, path, delta)
    }
    if (!store.moveIcons(finished.paths, targets)) {
      return
    }
    settlePaths = new Set(finished.paths)
    void glide.run({ x: loose.x - delta.x, y: loose.y - delta.y })
  }

  function settledDelta(finished: Drag, loose: Point): Point {
    const lead = store.iconPoint(finished.leadPath)
    if (lead === undefined) {
      return loose
    }
    const target = { x: lead.x + loose.x, y: lead.y + loose.y }
    const landing = store.landingFor(target, store.layout.iconSize, draggedRects(finished))
    return { x: landing.x - lead.x, y: landing.y - lead.y }
  }

  function draggedRects(finished: Drag): Rect[] {
    return finished.paths.flatMap((path) => {
      const point = store.iconPoint(path)
      if (point === undefined) {
        return []
      }
      return [rectOf(point, store.layout.iconSize)]
    })
  }

  function addTarget(targets: Map<string, Point>, path: string, delta: Point): void {
    const point = store.iconPoint(path)
    if (point === undefined) {
      return
    }
    targets.set(path, { x: point.x + delta.x, y: point.y + delta.y })
  }

  async function openMenu(entry: DesktopEntry, event: PointerEvent): Promise<void> {
    if (!store.isSelected(entry.path)) {
      store.selectOnly(entry.path)
    }
    store.openMenu(event.windowX, event.windowY, await iconMenu(entry))
  }

  async function iconMenu(entry: DesktopEntry): Promise<MenuItem[]> {
    const count = store.selection.size
    return [
      { label: openLabel(count), action: openSelection },
      { label: 'Open With', children: await handlerItems(entry) },
      { separator: true },
      { label: 'Rename', disabled: count > 1, action: () => beginRename(entry) },
      { label: trashLabel(count), danger: true, action: trashSelection },
    ]
  }

  function openLabel(count: number): string {
    if (count > 1) {
      return `Open ${count} items`
    }
    return 'Open'
  }

  function trashLabel(count: number): string {
    if (count > 1) {
      return `Move ${count} items to Trash`
    }
    return 'Move to Trash'
  }

  async function handlerItems(entry: DesktopEntry): Promise<MenuItem[]> {
    const reply = await bus.call('files:handlers', { path: entry.path }, CALL_TIMEOUT_MS)
    const handlers = handlersOf(reply)
    if (handlers.length === 0) {
      return [{ label: 'No applications found', disabled: true }]
    }
    return handlers.map((handler) => ({
      label: handler.name,
      action: () => openWith(entry, handler),
    }))
  }

  function openWith(entry: DesktopEntry, handler: Handler): void {
    void bus.call('files:openwith', { path: entry.path, command: handler.command }, CALL_TIMEOUT_MS)
  }

  function handlersOf(reply: unknown): Handler[] {
    return arrayOf(recordOf(reply).handlers).map((handler) => ({
      id: stringOf(handler.id),
      name: stringOf(handler.name),
      command: stringOf(handler.command),
    }))
  }

  async function beginRename(entry: DesktopEntry): Promise<void> {
    renameDraft = entry.name
    store.renamingPath = entry.path
    await tick()
    renameInput?.focus()
  }

  function commitRename(entry: DesktopEntry): void {
    const name = renameDraft.trim()
    store.renamingPath = ''
    if (name === '' || name === entry.name) {
      return
    }
    void bus.call('files:rename', { path: entry.path, name }, CALL_TIMEOUT_MS)
  }

  function trashSelection(): void {
    void bus.call('files:trash', { paths: [...store.selection] }, CALL_TIMEOUT_MS)
    store.clearSelection()
  }

  function tileBackground(path: string): string {
    if (store.isSelected(path)) {
      return alpha(colors.content, 0.2)
    }
    return 'transparent'
  }

  function tileBorder(path: string): string {
    if (store.isSelected(path)) {
      return alpha(colors.content, 0.25)
    }
    return 'transparent'
  }
</script>

{#snippet art(entry: DesktopEntry)}
  <View style={{ width: ICON_PIXELS, height: ICON_PIXELS, alignSelf: 'center' }}>
    {#if entry.image}
      <Image src={entry.path} fit="contain" style={{ width: ICON_PIXELS, height: ICON_PIXELS }} />
    {:else}
      <Icon name={entry.icon} fallback="text-x-generic" size={ICON_PIXELS} />
    {/if}
  </View>
{/snippet}

{#snippet caption(entry: DesktopEntry)}
  <Text style={{ fontSize: 11, textAlign: 'center', maxLines: 2, color: colors.content }}>{entry.name}</Text>
{/snippet}

{#each store.sortedEntries as entry (entry.path)}
  {@const point = shownPointOf(entry)}
  <Tooltip
    text={entry.name}
    side="bottom"
    disabled={drag !== null}
    anchorStyle={{ position: 'absolute', left: point.x, top: point.y }}
  >
    <View
      style={{
        width: store.layout.iconSize.width,
        height: store.layout.iconSize.height,
        gap: 4,
        padding: 7,
        radius: 8,
        borderWidth: 1,
        borderColor: tileBorder(entry.path),
        background: tileBackground(entry.path),
        transition: 120,
      }}
      hover={{ background: alpha(colors.content, 0.1) }}
      ondown={(event) => handleDown(entry, event)}
      onmove={handleMove}
      onup={() => handleUp(entry)}
    >
      {@render art(entry)}

      {#if store.renamingPath === entry.path}
        <Input
          bind:this={renameInput}
          bind:value={renameDraft}
          style={{ fontSize: 11, textAlign: 'center', paddingX: 4, paddingY: 0, radius: 4, background: colors.surface }}
          onsubmit={() => commitRename(entry)}
          onblur={() => commitRename(entry)}
        />
      {:else}
        {@render caption(entry)}
      {/if}
    </View>
  </Tooltip>
{/each}

<!-- The copies the pointer carries. The press keeps the pointer captured on
     the original tile, so these never take an event of their own. -->
{#each ghosts as entry (entry.path)}
  {@const point = ghostPointOf(entry)}
  <View
    style={{
      position: 'absolute',
      left: point.x,
      top: point.y,
      width: store.layout.iconSize.width,
      height: store.layout.iconSize.height,
      gap: 4,
      padding: 8,
      radius: 8,
      opacity: 0.6,
      background: alpha(colors.content, 0.12),
    }}
  >
    {@render art(entry)}
    {@render caption(entry)}
  </View>
{/each}
