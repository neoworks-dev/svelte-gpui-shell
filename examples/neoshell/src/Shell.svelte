<script lang="ts">
  import { monitors } from '@neoworks/svelte-gpui'
  import { subscribeTo } from './lib/bus'
  import type { Bus } from './host/bus'
  import type { ViewRegistry } from './host/plugins/views'
  import { isShellEnvironment } from './system/environment'
  import type { ShellEnvironment } from './system/environment'
  import { isDetached, keyOf, nodesOf, recordOf, resolve } from './tree'
  import type { TreeNode } from './tree'
  import ViewNode from './ViewNode.svelte'

  // The root: one layer surface per top-level view-tree node, opened on each
  // monitor the environment allows (or wherever the compositor chooses, when
  // it allows none in particular). The tree arrives on the retained "views"
  // topic, so a config edit adds, moves or removes a window without anything
  // here being reloaded.

  let { bus, registry }: { bus: Bus; registry: ViewRegistry } = $props()

  let nodes = $state<TreeNode[]>([])
  let environment = $state<ShellEnvironment | null>(null)
  // The registry is a plain object outside Svelte's reactivity. Every
  // registration bumps this, and the nodes re-resolve their type against it:
  // that is what makes a view appear when its extension finishes mounting.
  let generation = $state(0)

  $effect(() =>
    subscribeTo(bus, 'views', (message) => {
      nodes = nodesOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'shell.environment', (message) => {
      if (isShellEnvironment(message.data)) {
        environment = message.data
      }
    }),
  )

  $effect(() =>
    registry.onChange(() => {
      generation += 1
    }),
  )

  const targets = $derived(targetsOf(environment, monitors.list.map((monitor) => monitor.name)))

  // Nothing opens before the environment is known: a surface on the wrong
  // monitor cannot be taken back.
  function targetsOf(
    current: ShellEnvironment | null,
    connected: string[],
  ): Array<{ key: string; monitor: string | undefined }> {
    if (current === null) {
      return []
    }
    if (current.monitors === null) {
      return [{ key: 'default', monitor: undefined }]
    }
    return current.monitors
      .filter((name) => connected.includes(name))
      .map((name) => ({ key: name, monitor: name }))
  }

  // A window whose type has not been registered yet would be an empty surface
  // the compositor still maps, so the whole window waits for the component.
  function isReady(node: TreeNode, current: number): boolean {
    return resolve(registry, node.type, current) !== undefined
  }
</script>

{#each nodes as node, index (keyOf(node, index))}
  {#if isReady(node, generation)}
    {#if isDetached(recordOf(node.args))}
      <ViewNode {node} {bus} {registry} {generation} monitor={undefined} />
    {:else}
      {#each targets as target (target.key)}
        <ViewNode {node} {bus} {registry} {generation} monitor={target.monitor} />
      {/each}
    {/if}
  {/if}
{/each}
