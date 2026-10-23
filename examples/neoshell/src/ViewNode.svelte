<script lang="ts">
  import type { BusService } from './lib/bus'
  import type { ViewRegistry } from './host/plugins/views'
  import { provideSurface } from './surface'
  import { childrenOf, idOf, keyOf, resolve } from './tree'
  import type { TreeNode } from './tree'
  import Self from './ViewNode.svelte'

  // One view-tree node. The node's type resolves to a registered component and
  // its children render as that component's `children` snippet, so a view that
  // hosts others, like the bar holding the clock and the battery, takes them
  // the way any Svelte component takes its slot content. The surface context
  // is provided here, once, for the whole subtree.

  let {
    node,
    bus,
    registry,
    generation,
    monitor,
    plain = false,
  }: {
    node: TreeNode
    bus: BusService
    registry: ViewRegistry
    generation: number
    monitor?: string
    plain?: boolean
  } = $props()

  provideSurface({
    get monitor() {
      return monitor
    },
    get plain() {
      return plain
    },
    get name() {
      return surfaceName()
    },
  })

  const View = $derived(resolve(registry, node.type, generation))
  const children = $derived(childrenOf(node))

  function surfaceName(): string {
    const id = idOf(node)
    if (id === '') {
      return node.type
    }
    return id
  }
</script>

{#if View !== undefined}
  <View {bus} {registry} args={node.args} id={idOf(node)}>
    {#each children as child, index (keyOf(child, index))}
      <Self node={child} {bus} {registry} {generation} {monitor} {plain} />
    {/each}
  </View>
{/if}
