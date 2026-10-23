<script lang="ts">
  import { tick } from 'svelte'
  import { Button, Icon, Image, Input, Text, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import { subscribeTo } from '../../lib/bus'
  import type { BusService } from '../../lib/bus'
  import { clockTime, shortDate } from '../../lib/dateFormat'
  import { numberOf, recordOf, stringOf } from '../../lib/record'
  import { fileExists, homeDirectory, joinPath } from '../../system/fs'
  import { currentUser } from '../../system/user'
  import { initialsOf } from '../../system/userNames'
  import { alpha, colors } from '../../theme'
  import WallpaperLayer from '../neoshell/WallpaperLayer.svelte'
  import { checkPassword } from './auth'

  // One monitor's lock surface: the wallpaper, the clock, what is playing,
  // and the user with a password field. The layout is the macOS lock screen's
  // — time at the top, media in the middle, identity at the bottom — and
  // everything on it is a retained bus topic the rest of the shell already
  // publishes.

  interface Media {
    status: string
    title: string
    artist: string
    art: string
  }

  const AVATAR_PIXELS = 56
  const ART_PIXELS = 48
  const SCRIM_TOP: Style['background'] = {
    linear: { stops: [alpha('#000000', 0.45), alpha('#000000', 0.1)] },
  }
  const SCRIM_BOTTOM: Style['background'] = {
    linear: { stops: [alpha('#000000', 0.15), alpha('#000000', 0.55)] },
  }
  const HINT_IDLE = 'Enter Password'
  const HINT_WRONG = 'Wrong password'

  let { bus, onunlock }: { bus: BusService; onunlock: () => void } = $props()

  let now = $state(new Date())
  let userName = $state('')
  let realName = $state('')
  let batteryPercent = $state(-1)
  let batteryCharging = $state(false)
  let media = $state<Media>({ status: '', title: '', artist: '', art: '' })
  let avatar = $state('')
  let password = $state('')
  let hint = $state(HINT_IDLE)
  let failed = $state(false)
  let checking = $state(false)
  let field: Input | undefined = $state()

  const showMedia = $derived(media.title !== '' && media.status !== '' && media.status !== 'Stopped')
  const playIcon = $derived(playIconOf(media.status))
  const batteryIcon = $derived(batteryIconOf(batteryCharging))
  const hintColor = $derived(hintColorOf(failed))

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date()
    }, 1000)
    return () => clearInterval(timer)
  })

  $effect(() => {
    void loadUser()
  })

  $effect(() => {
    void tick().then(() => field?.focus())
  })

  $effect(() =>
    subscribeTo(bus, 'system.battery', (message) => {
      const data = recordOf(message.data)
      batteryPercent = Math.round(numberOf(data.percent, -1))
      batteryCharging = data.charging === true
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'media.player', (message) => {
      const data = recordOf(message.data)
      media = {
        status: stringOf(data.status),
        title: stringOf(data.title),
        artist: stringOf(data.artist),
        art: artPathOf(stringOf(data.artUrl)),
      }
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'config', (message) => {
      void loadAvatar(message.data)
    }),
  )

  async function loadUser(): Promise<void> {
    const identity = await currentUser()
    userName = identity.userName
    realName = identity.realName
  }

  async function submit(): Promise<void> {
    if (checking || password === '') {
      return
    }
    checking = true
    const result = await checkPassword(userName, password)
    checking = false
    password = ''
    if (result.ok) {
      onunlock()
      return
    }
    failed = true
    hint = HINT_WRONG
  }

  function typed(value: string): void {
    if (failed && value !== '') {
      failed = false
      hint = HINT_IDLE
    }
  }

  function control(command: string): void {
    void bus.call(command, {})
  }

  // lock.avatar in config, else the ~/.face every display manager reads.
  async function loadAvatar(snapshot: unknown): Promise<void> {
    const configured = stringOf(recordOf(recordOf(snapshot).lock).avatar)
    if (configured !== '' && (await fileExists(configured))) {
      avatar = configured
      return
    }
    const face = joinPath(await homeDirectory(), '.face')
    if (await fileExists(face)) {
      avatar = face
      return
    }
    avatar = ''
  }

  function artPathOf(artUrl: string): string {
    if (artUrl.startsWith('file://')) {
      return decodeURIComponent(artUrl.slice('file://'.length))
    }
    return ''
  }

  function playIconOf(status: string): string {
    if (status === 'Playing') {
      return 'media-playback-pause-symbolic'
    }
    return 'media-playback-start-symbolic'
  }

  function batteryIconOf(charging: boolean): string {
    if (charging) {
      return 'battery-good-charging-symbolic'
    }
    return 'battery-good-symbolic'
  }

  function hintColorOf(isFailed: boolean): string {
    if (isFailed) {
      return colors.error
    }
    return alpha(colors.content, 0.65)
  }
</script>

<View style={{ flex: 1, background: colors.canvas, color: colors.content }}>
  <WallpaperLayer {bus} />

  <View style={{ position: 'absolute', inset: 0 }}>
    <View style={{ flex: 1, background: SCRIM_TOP }}></View>
    <View style={{ flex: 1, background: SCRIM_BOTTOM }}></View>
  </View>

  <View style={{ flex: 1, paddingTop: 72, paddingBottom: 56, align: 'center' }}>
    {@render clock()}
    <View style={{ flex: 1 }}></View>
    {#if showMedia}
      {@render mediaCard()}
    {/if}
    <View style={{ flex: 1 }}></View>
    {@render user()}
  </View>
</View>

{#snippet clock()}
  <View style={{ align: 'center', gap: 2 }}>
    <Text style={{ fontSize: 22, fontWeight: 500, color: alpha(colors.content, 0.9) }}>{shortDate(now)}</Text>
    <Text style={{ fontSize: 108, fontWeight: 200, color: alpha(colors.content, 0.92) }}>{clockTime(now)}</Text>
    {#if batteryPercent >= 0}
      <View style={{ direction: 'row', align: 'center', gap: 6, color: alpha(colors.content, 0.75) }}>
        <Icon name={batteryIcon} size={16} />
        <Text style={{ fontSize: 13 }}>{batteryPercent}%</Text>
      </View>
    {/if}
  </View>
{/snippet}

{#snippet mediaCard()}
  <View
    style={{
      minWidth: 320,
      gap: 12,
      paddingX: 18,
      paddingY: 14,
      radius: 18,
      background: alpha(colors.elevated, 0.55),
      borderWidth: 1,
      borderColor: alpha(colors.content, 0.12),
    }}
  >
    <View style={{ direction: 'row', gap: 12, align: 'center' }}>
      <View
        style={{
          width: ART_PIXELS,
          height: ART_PIXELS,
          radius: 8,
          overflow: 'hidden',
          background: alpha(colors.content, 0.1),
        }}
      >
        {#if media.art !== ''}
          <Image src={media.art} fit="cover" style={{ width: ART_PIXELS, height: ART_PIXELS }} />
        {/if}
      </View>
      <View style={{ flex: 1, minWidth: 0, gap: 2 }}>
        <Text style={{ fontSize: 14, fontWeight: 600, truncate: true }}>{media.title}</Text>
        <Text style={{ fontSize: 12, color: alpha(colors.content, 0.7), truncate: true }}>{media.artist}</Text>
      </View>
    </View>
    <View style={{ direction: 'row', justify: 'center', gap: 28 }}>
      {@render mediaButton('media-skip-backward-symbolic', 'media:previous')}
      {@render mediaButton(playIcon, 'media:playPause')}
      {@render mediaButton('media-skip-forward-symbolic', 'media:next')}
    </View>
  </View>
{/snippet}

{#snippet mediaButton(icon: string, command: string)}
  <Button
    style={{ padding: 6, radius: 999, color: colors.content, transition: 120 }}
    hover={{ background: alpha(colors.content, 0.12) }}
    onpress={() => control(command)}
  >
    <Icon name={icon} size={18} />
  </Button>
{/snippet}

{#snippet user()}
  <View style={{ align: 'center', gap: 8 }}>
    {#if avatar !== ''}
      <View style={{ width: AVATAR_PIXELS, height: AVATAR_PIXELS, radius: AVATAR_PIXELS / 2, overflow: 'hidden' }}>
        <Image src={avatar} fit="cover" style={{ width: AVATAR_PIXELS, height: AVATAR_PIXELS }} />
      </View>
    {:else}
      <View
        style={{
          width: AVATAR_PIXELS,
          height: AVATAR_PIXELS,
          radius: AVATAR_PIXELS / 2,
          background: alpha(colors.content, 0.15),
          align: 'center',
          justify: 'center',
        }}
      >
        <Text style={{ fontSize: 20, fontWeight: 600 }}>{initialsOf(realName)}</Text>
      </View>
    {/if}
    <Text style={{ fontSize: 15, fontWeight: 600, color: alpha(colors.content, 0.95) }}>{realName}</Text>
    <Input
      bind:this={field}
      bind:value={password}
      secure
      placeholder="Enter Password"
      oninput={typed}
      onsubmit={() => void submit()}
      style={{
        width: 220,
        height: 30,
        paddingX: 12,
        radius: 999,
        fontSize: 13,
        color: colors.content,
        background: alpha(colors.content, 0.14),
        borderWidth: 1,
        borderColor: alpha(colors.content, 0.18),
      }}
      focus={{ borderColor: alpha(colors.content, 0.4) }}
    />
    <Text style={{ fontSize: 12, color: hintColor }}>{hint}</Text>
  </View>
{/snippet}
