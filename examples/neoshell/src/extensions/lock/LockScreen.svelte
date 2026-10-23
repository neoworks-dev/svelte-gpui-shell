<script lang="ts">
  import { monitors, sessionLock } from '@neoworks/svelte-gpui'
  import { Window } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import { recordOf } from '../../lib/record'
  import type { ViewProps } from '../../host/plugins/views'
  import { isShellEnvironment, lockMonitorsOf } from '../../system/environment'
  import type { ShellEnvironment } from '../../system/environment'
  import LockSurface from './LockSurface.svelte'

  // The lock screen's windows. A detached view: it renders no window of its
  // own — while the session is locked it puts one session-lock surface on
  // every monitor the shell runs on, and while it is not it renders nothing.
  //
  // Mounting the windows is what locks the session and unmounting every one
  // is what unlocks it, so the windows follow the retained lock.state. When
  // the compositor refuses or ends the lock (another locker holds it) or the
  // host will not open lock windows, sessionLock.state turns "finished" and
  // the state is cleared, which unmounts them.

  let { bus }: ViewProps = $props()

  let locked = $state(false)
  let environment = $state<ShellEnvironment | null>(null)

  const targets = $derived(lockMonitorsOf(environment, monitors.list.map((monitor) => monitor.name)))

  $effect(() =>
    subscribeTo(bus, 'lock.state', (message) => {
      locked = recordOf(message.data).locked === true
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'shell.environment', (message) => {
      if (isShellEnvironment(message.data)) {
        environment = message.data
      }
    }),
  )

  $effect(() => {
    if (locked && sessionLock.state === 'finished') {
      requestUnlock()
    }
  })

  function requestUnlock(): void {
    bus.publish('lock:unlock', {})
  }
</script>

{#if locked}
  {#each targets as monitor (monitor)}
    <Window lock {monitor}>
      <LockSurface {bus} onunlock={requestUnlock} />
    </Window>
  {/each}
{/if}
