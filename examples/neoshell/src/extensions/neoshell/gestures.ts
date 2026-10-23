import type { PointerEvent } from '@neoworks/svelte-gpui/ui'
import type { Point } from './freeform'

// What the desktop's pointer handling asks of an event: which button it was,
// whether it extends a selection, and how far a held button has travelled.

export const PRIMARY_BUTTON = 0
export const SECONDARY_BUTTON = 2

export const DRAG_THRESHOLD_PX = 4

// Whether the click extends a selection rather than replacing it.
export function extendsSelection(event: PointerEvent): boolean {
  return event.ctrl
}

export function windowPointOf(event: PointerEvent): Point {
  return { x: event.windowX, y: event.windowY }
}

export function travelOf(start: Point, event: PointerEvent): Point {
  return { x: event.windowX - start.x, y: event.windowY - start.y }
}

export function hasTravelled(travel: Point): boolean {
  return Math.abs(travel.x) > DRAG_THRESHOLD_PX || Math.abs(travel.y) > DRAG_THRESHOLD_PX
}
