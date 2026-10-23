import { getContext, setContext } from 'svelte'

// What a view needs to know about the window it is drawn in, handed down by
// the shell so a view does not ask where it is.
//
//   monitor  the output the window opens on; undefined lets the compositor pick
//   plain    draw the surface as a view in the current window instead of a
//            layer surface, which is how the smoke harness shows every view
//   name     the view tree node's id, which names the layer namespace

export interface SurfaceContext {
  monitor: string | undefined
  plain: boolean
  name: string
}

const KEY = Symbol('neoshell.surface')

export function provideSurface(surface: SurfaceContext): void {
  setContext(KEY, surface)
}

export function surfaceOf(): SurfaceContext {
  const surface = getContext<SurfaceContext | undefined>(KEY)
  if (surface === undefined) {
    return { monitor: undefined, plain: true, name: 'neoshell' }
  }
  return surface
}
