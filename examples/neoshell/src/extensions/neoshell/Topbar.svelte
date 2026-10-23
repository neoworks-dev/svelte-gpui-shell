<script lang="ts">
  import { subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { Button, Text, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import Surface from '../../Surface.svelte'
  import { numberOf, recordOf } from '../../lib/record'
  import { alpha, colors } from '../../theme'
  import {
    activeIdOf,
    urgentAddressOf,
    windowWorkspacesOf,
    workspacesOf,
  } from './hypr'
  import type { WorkspaceEntry } from './hypr'

  // The top bar: workspaces on the left, the configured children (battery,
  // clock, …) on the right, and a reserved gap between them — the notch is a
  // sibling top-level view floating over the bar's centre, so the bar must keep
  // that span clear.

  let { bus, args, children }: ViewProps = $props()

  const TOOLTIP_AREA = 40
  const DEFAULT_BAR_HEIGHT = 30

  const centerGap = $derived(centerGapOf(args))
  const barHeight = $derived(numberOf(recordOf(args).height, DEFAULT_BAR_HEIGHT))

  let workspaceList = $state<WorkspaceEntry[]>([])
  let activeWorkspaceId = $state(-1)
  let urgentWorkspaceIds = $state<ReadonlySet<number>>(new Set())
  let workspaceByWindowAddress = new Map<string, number>()

  // The subscriptions are set up once: their handlers run long after the effect
  // has finished, so nothing they read becomes a dependency.
  $effect(() =>
    subscribeTo(bus, 'hypr.workspaces', (message) => {
      workspaceList = workspacesOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.activeworkspace', (message) => {
      activeWorkspaceId = activeIdOf(message.data)
      urgentWorkspaceIds = without(urgentWorkspaceIds, activeWorkspaceId)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.windows', (message) => {
      workspaceByWindowAddress = windowWorkspacesOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.event', (message) => {
      markUrgent(urgentAddressOf(message.data))
    }),
  )

  // Urgency arrives out of band as "urgent>>ADDRESS"; the address is resolved
  // against the window snapshot to colour the workspace that owns it.
  function markUrgent(address: string | null): void {
    if (address === null) {
      return
    }
    const workspaceId = workspaceByWindowAddress.get(address)
    if (workspaceId === undefined || workspaceId === activeWorkspaceId) {
      return
    }
    urgentWorkspaceIds = new Set(urgentWorkspaceIds).add(workspaceId)
  }

  function without(set: ReadonlySet<number>, id: number): ReadonlySet<number> {
    if (!set.has(id)) {
      return set
    }
    const remaining = new Set(set)
    remaining.delete(id)
    return remaining
  }

  function switchWorkspace(workspace: WorkspaceEntry): void {
    void bus.call('hypr:dispatch', { dispatcher: 'workspace', arg: String(workspace.id) })
  }

  const WORKSPACE_STYLES: Record<string, Style> = {
    urgent: { background: alpha(colors.warning, 0.2), color: colors.warning },
    active: { background: alpha(colors.content, 0.2), color: colors.content },
    occupied: { color: alpha(colors.content, 0.85) },
    empty: { color: alpha(colors.content, 0.45) },
  }

  const workspaceBase: Style = {
    minHeight: 20,
    minWidth: 12,
    paddingX: 6,
    radius: 6,
    fontSize: 11,
    fontWeight: 600,
    justify: 'center',
    align: 'center',
    transition: 150,
  }

  function stateOf(workspace: WorkspaceEntry): string {
    if (urgentWorkspaceIds.has(workspace.id)) {
      return 'urgent'
    }
    if (workspace.id === activeWorkspaceId) {
      return 'active'
    }
    if (workspace.occupied) {
      return 'occupied'
    }
    return 'empty'
  }

  // centerGap is the span the bar keeps clear for the notch, which floats over
  // the bar's centre as its own layer-shell window.
  function centerGapOf(value: unknown): number {
    const gap = recordOf(value).centerGap
    if (typeof gap !== 'number') {
      return 0
    }
    return gap
  }


</script>

<Surface
  {args}
  extraHeight={TOOLTIP_AREA}
  style={{
    width: '100%',
    height: '100%',
  }}
>
  <View
    input
    style={{
      height: barHeight,
      direction: 'row',
      align: 'center',
      paddingX: 12,
      background: alpha(colors.elevated, 0.4),
      color: colors.content,
      fontSize: 13,
      fontWeight: 500,
    }}
  >
    <View style={{ flex: 1, direction: 'row', align: 'center', gap: 4 }}>
      {#each workspaceList as workspace (workspace.id)}
        <Button
          style={{ ...workspaceBase, ...WORKSPACE_STYLES[stateOf(workspace)] }}
          hover={{ background: alpha(colors.content, 0.12) }}
          onpress={() => switchWorkspace(workspace)}
        >
          <Text>{workspace.name}</Text>
        </Button>
      {/each}
    </View>

    <!-- The notch gap: a fixed-width centre slot that keeps the sides apart. -->
    <View style={{ width: centerGap }}></View>

    <View style={{ flex: 1, direction: 'row', align: 'center', justify: 'end', gap: 16 }}>
      {@render children?.()}
    </View>
  </View>
</Surface>
