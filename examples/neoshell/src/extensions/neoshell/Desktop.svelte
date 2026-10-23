<script lang="ts">
  import { untrack } from 'svelte'
  import { View } from '@neoworks/svelte-gpui/ui'
  import type { PointerEvent } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { arrayOf, recordOf, stringOf } from '../../lib/record'
  import Surface from '../../Surface.svelte'
  import { alpha, colors, windowBackground } from '../../theme'
  import ContextMenu from './ContextMenu.svelte'
  import DesktopIcons from './DesktopIcons.svelte'
  import WidgetSlot from './WidgetSlot.svelte'
  import { DesktopStore } from './desktopStore.svelte'
  import type { MenuItem } from './desktopStore.svelte'
  import type { Point, Size } from './freeform'
  import { SECONDARY_BUTTON, windowPointOf } from './gestures'

  // The desktop: the background-layer view that owns everything behind the
  // windows — the desktop folder's icons and the widget grid. Icons and widgets
  // share one grid, so this node also owns what spans both: the rubber band
  // that selects icons, the background context menu, and the widget placement
  // both halves resolve against.
  //
  // Everything is positioned absolutely in the window, which is the output, so
  // window coordinates are desktop coordinates.

  const CALL_TIMEOUT_MS = 10000

  interface Band {
    start: Point
    x: number
    y: number
    width: number
    height: number
  }

  let { bus, registry, args }: ViewProps = $props()

  // The bus is the shell's own and does not change for the life of the view.
  const store = new DesktopStore(untrack(() => bus))

  let band = $state<Band | null>(null)
  let preview = $state<{ point: Point; box: Size; allowed: boolean } | null>(null)
  // The registry is outside Svelte's reactivity, so a widget provider that
  // registers after the desktop is up is picked up by bumping this.
  let generation = $state(0)

  $effect(() => {
    store.setSeeds(recordOf(args).widgets)
  })

  $effect(() =>
    subscribeTo(bus, 'config', (message) => {
      store.applyConfig(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'widgets:add', (message) => {
      const request = recordOf(message.data)
      store.addWidget(stringOf(request.type), stringOf(request.size), droppedPoint(request))
    }),
  )

  $effect(() =>
    registry.onChange(() => {
      generation += 1
    }),
  )

  // Config is the desktop's state file, not just its seed: whatever the
  // resolved layout ends up being — icons the folder flowed into place, a
  // widget displaced off a stale cell — is written back once it settles.
  $effect(() => {
    store.schedulePersist(store.layout)
    return () => store.cancelPersist()
  })

  // The gallery is a different window, so a widget added from it arrives as
  // coordinates rather than as a drop event. Both cover the output, so the
  // coordinates need no translation.
  function droppedPoint(request: Record<string, unknown>): Point | undefined {
    const { x, y } = request
    if (typeof x !== 'number' || typeof y !== 'number') {
      return undefined
    }
    return { x, y }
  }

  function handleDown(event: PointerEvent): void {
    store.clearSelection()
    if (event.button === SECONDARY_BUTTON) {
      void openMenu(event.windowX, event.windowY)
      return
    }
    const start = windowPointOf(event)
    band = { start, x: start.x, y: start.y, width: 0, height: 0 }
  }

  function moveBand(event: PointerEvent): void {
    if (band === null) {
      return
    }
    const { start } = band
    band = {
      start,
      x: Math.min(start.x, event.windowX),
      y: Math.min(start.y, event.windowY),
      width: Math.abs(event.windowX - start.x),
      height: Math.abs(event.windowY - start.y),
    }
    store.selectPaths(pathsInBand(band))
  }

  function endBand(): void {
    band = null
  }

  // Icon cells are known geometry, so the band is intersected against computed
  // rects rather than against measured widgets.
  function pathsInBand(current: Band): string[] {
    const hit: string[] = []
    for (const [path, point] of store.layout.icons) {
      if (intersects(current, point)) {
        hit.push(path)
      }
    }
    return hit
  }

  function intersects(current: Band, point: Point): boolean {
    const { width, height } = store.layout.iconSize
    if (point.x + width < current.x || point.x > current.x + current.width) {
      return false
    }
    return !(point.y + height < current.y || point.y > current.y + current.height)
  }

  async function openMenu(x: number, y: number): Promise<void> {
    store.openMenu(x, y, await backgroundMenu())
  }

  async function backgroundMenu(): Promise<MenuItem[]> {
    return [
      { label: 'Edit Widgets', action: () => store.openGallery() },
      { separator: true },
      { label: 'New Folder', action: newFolder },
      { separator: true },
      {
        label: 'Sort Icons',
        children: [
          {
            label: 'By Name',
            checked: store.sortMode === 'name',
            action: () => store.sortIcons('name'),
          },
          {
            label: 'By Type',
            checked: store.sortMode === 'type',
            action: () => store.sortIcons('type'),
          },
        ],
      },
      { label: 'Clean Up Icons', action: () => store.cleanUpIcons() },
      { label: 'Select All', action: () => store.selectAll() },
      { separator: true },
      { label: 'Change Wallpaper', children: await wallpaperItems() },
      {
        label: 'Lock Desktop',
        checked: store.desktopLocked,
        action: () => store.setDesktopLocked(!store.desktopLocked),
      },
    ]
  }

  async function wallpaperItems(): Promise<MenuItem[]> {
    const reply = await bus.call('files:images', {}, CALL_TIMEOUT_MS)
    const images = arrayOf(recordOf(reply).entries)
    if (images.length === 0) {
      return [{ label: 'No images in the wallpaper folder', disabled: true }]
    }
    return images.map((image) => ({
      label: stringOf(image.name),
      action: () => setWallpaper(stringOf(image.path)),
    }))
  }

  function setWallpaper(path: string): void {
    void bus.call('config:set', { key: 'appearance.wallpaper', value: path }, CALL_TIMEOUT_MS)
  }

  function newFolder(): void {
    void bus.call('files:newfolder', {}, CALL_TIMEOUT_MS)
  }

  function showPreview(point: Point | null, box: Size, allowed: boolean): void {
    if (point === null) {
      preview = null
      return
    }
    preview = { point, box, allowed }
  }

  function previewBackground(allowed: boolean): string {
    if (allowed) {
      return alpha(colors.content, 0.12)
    }
    return alpha(colors.content, 0.04)
  }

  function previewBorder(allowed: boolean): string {
    if (allowed) {
      return alpha(colors.content, 0.25)
    }
    return alpha(colors.content, 0.1)
  }
</script>

<Surface {args} style={{ background: windowBackground }}>
  <!-- What is not an icon or a widget falls through to the root, which takes
       the rubber band and the menu. -->
  <View
    style={{ flex: 1 }}
    onresize={(layout) => store.setViewport(layout.width, layout.height)}
    ondown={handleDown}
    onmove={moveBand}
    onup={endBand}
  >
    {#if preview !== null}
      <View
        style={{
          position: 'absolute',
          left: preview.point.x,
          top: preview.point.y,
          width: preview.box.width,
          height: preview.box.height,
          radius: 24,
          background: previewBackground(preview.allowed),
          borderWidth: 1,
          borderColor: previewBorder(preview.allowed),
        }}
      ></View>
    {/if}

    {#each store.widgetPlacements() as placement (placement.id)}
      <WidgetSlot {placement} {bus} {registry} {generation} {store} onpreview={showPreview} />
    {/each}

    <!-- Icons come after the widgets so a dragged icon travels over them. -->
    <DesktopIcons {bus} {store} />

    {#if band !== null}
      <View
        style={{
          position: 'absolute',
          left: band.x,
          top: band.y,
          width: band.width,
          height: band.height,
          radius: 2,
          background: alpha(colors.accent, 0.2),
          borderWidth: 1,
          borderColor: alpha(colors.accent, 0.7),
        }}
      ></View>
    {/if}

    {#if store.menu !== null}
      {#key store.menu}
        <ContextMenu menu={store.menu} onclose={() => store.closeMenu()} />
      {/key}
    {/if}
  </View>
</Surface>
