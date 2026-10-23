import { Tween } from 'svelte/motion'
import { cubicOut } from 'svelte/easing'
import type { Point } from './freeform'

// A dropped item does not teleport to where it settles: it is drawn offset
// back to where the pointer let it go and the offset eases to nothing. Callers
// add `offset` to the item's own point.

const GLIDE_MS = 180
const NO_OFFSET: Point = { x: 0, y: 0 }

export class Glide {
  private readonly tween = new Tween<Point>(NO_OFFSET, { duration: GLIDE_MS, easing: cubicOut })
  private running = $state(false)
  private generation = 0

  get offset(): Point {
    if (!this.running) {
      return NO_OFFSET
    }
    return this.tween.current
  }

  get active(): boolean {
    return this.running
  }

  async run(from: Point): Promise<void> {
    this.generation += 1
    const mine = this.generation
    this.running = true
    await this.tween.set(from, { duration: 0 })
    await this.tween.set(NO_OFFSET)
    if (mine === this.generation) {
      this.running = false
    }
  }

  stop(): void {
    this.generation += 1
    this.running = false
  }
}
