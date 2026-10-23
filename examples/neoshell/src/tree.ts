import type { ViewComponent, ViewRegistry } from './host/plugins/views.js'

// The shape of the config view tree, and the arg readers the shell needs to
// turn a top-level node into a layer-shell window.

export interface TreeNode {
  id?: string
  type: string
  args?: unknown
  children?: TreeNode[]
}

export type Layer = 'background' | 'bottom' | 'top' | 'overlay'
export type Edge = 'top' | 'bottom' | 'left' | 'right'
export type KeyboardMode = 'none' | 'on-demand' | 'exclusive'

const LAYERS = new Set<string>(['background', 'bottom', 'top', 'overlay'])
const EDGES: Edge[] = ['top', 'bottom', 'left', 'right']

export function nodesOf(data: unknown): TreeNode[] {
  if (!Array.isArray(data)) {
    return []
  }
  return data.filter(isTreeNode)
}

export function childrenOf(node: TreeNode): TreeNode[] {
  if (node.children === undefined) {
    return []
  }
  return node.children
}

// A node's key has to survive a config edit that reorders the tree, so it is
// the id when there is one and the position-qualified type when there is not.
export function keyOf(node: TreeNode, index: number): string {
  if (node.id !== undefined && node.id !== '') {
    return node.id
  }
  return `${index}:${node.type}`
}

export function idOf(node: TreeNode): string {
  if (node.id === undefined) {
    return ''
  }
  return node.id
}

// The registry is not reactive on its own; the shell bumps a generation
// counter from onChange and passes it in, so resolving a type again is what a
// late registration triggers.
export function resolve(
  registry: ViewRegistry,
  type: string,
  _generation: number,
): ViewComponent | undefined {
  return registry.resolve(type)
}

export function recordOf(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null) {
    return {}
  }
  return value as Record<string, unknown>
}

export function namespaceOf(node: TreeNode): string {
  const id = idOf(node)
  if (id === '') {
    return `neoshell.${node.type}`
  }
  return `neoshell.${id}`
}

// A detached node renders no window of its own: its view creates and owns
// whatever toplevels it needs, such as the lock screen's one surface per
// monitor.
export function isDetached(args: Record<string, unknown>): boolean {
  return args.detached === true
}

export function layerOf(args: Record<string, unknown>): Layer {
  const layer = args.layer
  if (typeof layer === 'string' && LAYERS.has(layer)) {
    return layer as Layer
  }
  return 'top'
}

// The config spells the on-demand mode without a hyphen.
export function keyboardOf(args: Record<string, unknown>): KeyboardMode {
  const keyboard = args.keyboard
  if (keyboard === 'ondemand') {
    return 'on-demand'
  }
  if (keyboard === 'exclusive') {
    return 'exclusive'
  }
  return 'none'
}

// A node that names no edges is anchored to every one, which is what a
// full-screen desktop layer wants.
export function anchorOf(args: Record<string, unknown>): Edge[] {
  if (!Array.isArray(args.anchors)) {
    return [...EDGES]
  }
  return args.anchors.filter((edge): edge is Edge => EDGES.includes(edge as Edge))
}

// The compositor reserves this much space along the anchored edge. -1 is the
// default rather than 0, because it means "reserve nothing and ignore what
// everyone else reserved": a notch floating over the bar has to overlap the
// strip the bar reserved, and 0 would have the compositor push it clear.
export function exclusiveZoneOf(args: Record<string, unknown>): number {
  const size = args.exclusiveSize
  if (typeof size !== 'number') {
    return -1
  }
  return size
}

export function marginOf(args: Record<string, unknown>): number {
  const margin = args.margin
  if (typeof margin !== 'number') {
    return 0
  }
  return margin
}

// 0 leaves the axis to the anchors: a layer surface anchored to both edges of
// an axis is stretched along it.
export function sizeOf(args: Record<string, unknown>, key: 'width' | 'height'): number {
  const size = args[key]
  if (typeof size !== 'number' || size < 0) {
    return 0
  }
  return size
}

function isTreeNode(value: unknown): value is TreeNode {
  if (typeof value !== 'object' || value === null) {
    return false
  }
  return typeof (value as TreeNode).type === 'string'
}
