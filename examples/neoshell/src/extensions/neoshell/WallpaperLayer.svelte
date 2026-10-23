<script lang="ts">
  import { Image, Video, View } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import type { BusService } from '../../lib/bus'
  import { recordOf } from '../../lib/record'
  import {
    DEFAULT_LIVE_OPTIONS,
    fullscreenShowing,
    isVideoPath,
    liveOptionsOf,
    onBatteryPower,
    shouldPlay,
  } from './liveWallpaper'

  // The wallpaper itself, for whatever window draws it: the configured image
  // or video, and nothing when none is set.
  //
  // A video path is a live wallpaper: a looping, muted <Video>. It holds still
  // while a fullscreen window is on screen and, if configured, while the
  // machine is on battery — there is nothing to look at in the first case and
  // every frame costs in the second. Both rules and the playback speed live
  // under appearance.liveWallpaper.

  let { bus }: { bus: BusService } = $props()

  let wallpaperPath = $state('')
  let options = $state(DEFAULT_LIVE_OPTIONS)
  let workspaces = $state.raw<unknown>(null)
  let monitors = $state.raw<unknown>(null)
  let battery = $state.raw<unknown>(null)

  $effect(() =>
    subscribeTo(bus, 'config', (message) => {
      wallpaperPath = pathOf(message.data)
      options = liveOptionsOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.workspaces', (message) => {
      workspaces = message.data
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.monitors', (message) => {
      monitors = message.data
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'system.battery', (message) => {
      battery = message.data
    }),
  )

  const isLive = $derived(isVideoPath(wallpaperPath))
  const paused = $derived(
    !shouldPlay(options, fullscreenShowing(workspaces, monitors), onBatteryPower(battery)),
  )

  function pathOf(snapshot: unknown): string {
    const appearance = recordOf(recordOf(snapshot).appearance)
    if (typeof appearance.wallpaper === 'string') {
      return appearance.wallpaper
    }
    return ''
  }
</script>

{#if isLive}
  <Video
    src={wallpaperPath}
    fit="cover"
    {paused}
    rate={options.speed}
    style={{ position: 'absolute', inset: 0 }}
    onerror={(error) => console.warn('wallpaper: cannot play', wallpaperPath, error.message)}
  />
{:else if wallpaperPath !== ''}
  <View style={{ position: 'absolute', inset: 0 }}>
    <Image src={wallpaperPath} fit="cover" style={{ width: '100%', height: '100%' }} />
  </View>
{/if}
