<script lang="ts">
  import { View } from '@neoworks/svelte-gpui/ui'
  import type { PointerEvent } from '@neoworks/svelte-gpui/ui'
  import type { BusService } from '../../lib/bus'
  import type { ViewComponent, ViewRegistry } from '../../host/plugins/views'
  import { alpha } from '../../theme'
  import { SIZE_NAMES } from './freeform'
  import type { Point, Rect, Size } from './freeform'
  import type { DesktopStore, MenuItem, WidgetPlacement } from './desktopStore.svelte'
  import { Glide } from './glide.svelte'
  import {
    hasTravelled,
    PRIMARY_BUTTON,
    SECONDARY_BUTTON,
    travelOf,
    windowPointOf,
  } from './gestures'

  // One widget on the desktop: the registered view for its type, placed by the
  // store and draggable unless it or the desktop is locked.
  //
  // A drag lifts the widget and lets it follow the pointer exactly, while a
  // placeholder marks where it would settle; on release it glides into that
  // spot — or back to where it came from when the spot is taken.

  const LIFT_SCALE = 1.03

  interface Drag {
    start: Point
    dx: number
    dy: number
    moved: boolean
  }

  let {
    placement,
    bus,
    registry,
    generation,
    store,
    onpreview,
  }: {
    placement: WidgetPlacement
    bus: BusService
    registry: ViewRegistry
    generation: number
    store: DesktopStore
    onpreview: (point: Point | null, box: Size, allowed: boolean) => void
  } = $props()

  let drag = $state<Drag | null>(null)
  const glide = new Glide()

  // Reading `generation` is what re-resolves the type when a widget provider's
  // views register after the desktop is already up.
  const Card = $derived(viewFor(placement.type, generation))
  const point = $derived(shownPoint(placement.point, drag))
  const lifted = $derived(drag !== null && drag.moved)

  function viewFor(type: string, _generation: number): ViewComponent | undefined {
    return registry.resolve(type)
  }

  function shownPoint(home: Point, current: Drag | null): Point {
    if (current !== null && current.moved) {
      return { x: home.x + current.dx, y: home.y + current.dy }
    }
    return { x: home.x + glide.offset.x, y: home.y + glide.offset.y }
  }

  function handleDown(event: PointerEvent): void {
    if (event.button === SECONDARY_BUTTON) {
      store.openMenu(event.windowX, event.windowY, menuItems())
      return
    }
    if (event.button !== PRIMARY_BUTTON || !store.widgetDraggable(placement.id)) {
      return
    }
    glide.stop()
    drag = { start: windowPointOf(event), dx: 0, dy: 0, moved: false }
  }

  // The widget follows the pointer; the placeholder shows where it would
  // land, which is the store's answer — so the outline and the drop agree.
  function handleMove(event: PointerEvent): void {
    if (drag === null) {
      return
    }
    const travel = travelOf(drag.start, event)
    const moved = drag.moved || hasTravelled(travel)
    drag = { ...drag, dx: travel.x, dy: travel.y, moved }
    if (moved) {
      const landing = landingFor(looseOf(drag))
      onpreview(landing, placement.box, store.widgetDropAllowed(placement.id, landing))
    }
  }

  function handleUp(): void {
    if (drag === null) {
      return
    }
    const finished = drag
    drag = null
    onpreview(null, placement.box, false)
    if (finished.moved) {
      dropWidget(looseOf(finished))
    }
  }

  function looseOf(current: Drag): Point {
    return { x: placement.point.x + current.dx, y: placement.point.y + current.dy }
  }

  // A refused drop — the pixels are taken — leaves the store as it was, and
  // the widget glides back to the point it still holds.
  function dropWidget(loose: Point): void {
    const landing = landingFor(loose)
    const accepted = store.moveWidget(placement.id, landing)
    const home = settledPoint(accepted, landing)
    void glide.run({ x: loose.x - home.x, y: loose.y - home.y })
  }

  function settledPoint(accepted: boolean, landing: Point): Point {
    if (accepted) {
      return landing
    }
    return placement.point
  }

  function landingFor(loose: Point): Point {
    return store.landingFor(loose, placement.box, [rectOfPlacement()])
  }

  function rectOfPlacement(): Rect {
    return {
      x: placement.point.x,
      y: placement.point.y,
      width: placement.box.width,
      height: placement.box.height,
    }
  }

  function menuItems(): MenuItem[] {
    return [
      { label: 'Edit Widgets', action: () => store.openGallery() },
      { separator: true },
      {
        label: 'Locked',
        checked: placement.locked,
        disabled: store.desktopLocked,
        action: () => store.setWidgetLocked(placement.id, !placement.locked),
      },
      { label: 'Size', children: sizeItems() },
      { separator: true },
      { label: 'Remove Widget', danger: true, action: () => store.removeWidget(placement.id) },
    ]
  }

  function sizeItems(): MenuItem[] {
    return SIZE_NAMES.map((size) => ({
      label: capitalized(size),
      checked: size === placement.size,
      action: () => store.setWidgetSize(placement.id, size),
    }))
  }

  function capitalized(value: string): string {
    return value.slice(0, 1).toUpperCase() + value.slice(1)
  }

  function liftedShadow(isLifted: boolean) {
    if (isLifted) {
      return { y: 25, blur: 50, color: alpha('#000000', 0.55) }
    }
    return null
  }

  function liftedScale(isLifted: boolean): number {
    if (isLifted) {
      return LIFT_SCALE
    }
    return 1
  }
</script>

{#if Card !== undefined}
  <!-- The card is told its own size rather than measuring it: the store
       already holds it, and a change from the widget's context menu reaches
       the card as a prop instead of as a resize it has to observe. -->
  <View
    style={{
      position: 'absolute',
      left: point.x,
      top: point.y,
      width: placement.box.width,
      height: placement.box.height,
      radius: 24,
      scale: liftedScale(lifted),
      shadow: liftedShadow(lifted),
    }}
    ondown={handleDown}
    onmove={handleMove}
    onup={handleUp}
  >
    <Card
      {bus}
      {registry}
      args={{ size: placement.size, width: placement.box.width, height: placement.box.height }}
      id={placement.id}
    />
  </View>
{/if}
