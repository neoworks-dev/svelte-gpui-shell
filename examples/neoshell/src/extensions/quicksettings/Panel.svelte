<script lang="ts">
  import { errorOf, subscribeTo } from '../../lib/bus'
  import type { ViewProps } from '../../host/plugins/views'
  import { arrayOf, numberOf, recordOf, stringListOf, stringOf } from '../../lib/record'
  import { Button, Icon, Presence, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import { fade, fly } from 'svelte/transition'
  import Surface from '../../Surface.svelte'
  import { alpha, colors, windowBackground } from '../../theme'
  import Tile from './Tile.svelte'
  import SliderRow from './SliderRow.svelte'
  import WifiPage from './WifiPage.svelte'
  import BluetoothPage from './BluetoothPage.svelte'
  import { deviceIcon, iconName, signalIcon, signalLabel } from './icons'
  import { PANEL_WIDTH, panelStyle, separatorStyle } from './look'
  import type { BluetoothDevice, BluetoothState, NetworkState, WifiNetwork } from './types'
  //
  // The quick settings panel: a ChromeOS-style tray hanging under the bar,
  // opened by the clock.
  //
  //   quicksettings:toggle / :open / :close   drive visibility; :open takes
  //                                           {page: 'wifi' | 'bluetooth'}
  //
  // The tray's window spans the output. While the tray is closed its input
  // region is empty, so the output stays click-through; while open, a backdrop
  // under the panel takes the click that lands anywhere else and dismisses it,
  // the way a menu closes when clicked away from.
  //
  //   args.offsetTop   the bar's height plus the gap the panel hangs below it
  //   args.offsetEnd   the gap between the panel and the output's right edge

  const COMMAND_TIMEOUT_MS = 5000
  const SCREENSHOT_COMMAND = 'grim -g "$(slurp)"'

  type Page = 'main' | 'wifi' | 'bluetooth'

  let { bus, args }: ViewProps = $props()

  const offsetTop = $derived(numberOf(recordOf(args).offsetTop, 36))
  const offsetEnd = $derived(numberOf(recordOf(args).offsetEnd, 6))

  let open = $state(false)
  let page = $state<Page>('main')
  let powerMenuOpen = $state(false)

  let network = $state<NetworkState>(emptyNetworkState())
  let networks = $state<WifiNetwork[]>([])
  let bluetooth = $state<BluetoothState>(emptyBluetoothState())
  let devices = $state<BluetoothDevice[]>([])
  let volume = $state(0)
  let muted = $state(false)
  let brightness = $state(0)
  let brightnessAvailable = $state(false)
  let batteryPercent = $state(-1)
  let batteryStatus = $state('')
  let doNotDisturb = $state(false)
  let keyboardLayout = $state('')

  const connectedDevice = $derived(devices.find((device) => device.connected))

  $effect(() => subscribeTo(bus, 'quicksettings:toggle', () => setOpen(!open)))
  $effect(() =>
    subscribeTo(bus, 'quicksettings:open', (message) => {
      setOpen(true)
      openPage(recordOf(message.data).page)
    }),
  )
  $effect(() => subscribeTo(bus, 'quicksettings:close', () => setOpen(false)))

  $effect(() =>
    subscribeTo(bus, 'network.state', (message) => {
      network = networkStateOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'network.networks', (message) => {
      networks = networksOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'bluetooth.state', (message) => {
      bluetooth = bluetoothStateOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'bluetooth.devices', (message) => {
      devices = devicesOf(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'system.volume', (message) => {
      applyVolume(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'system.brightness', (message) => {
      applyBrightness(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'system.battery', (message) => {
      applyBattery(message.data)
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'config', (message) => {
      doNotDisturb = recordOf(recordOf(message.data).notifications).doNotDisturb === true
    }),
  )

  $effect(() =>
    subscribeTo(bus, 'hypr.event', (message) => {
      applyKeyboardLayout(message.data)
    }),
  )

  function setOpen(next: boolean): void {
    open = next
    if (next) {
      return
    }
    page = 'main'
    powerMenuOpen = false
  }

  function openPage(requested: unknown): void {
    if (requested === 'wifi' || requested === 'bluetooth') {
      page = requested
    }
  }

  function keepPress(): void {}

  function close(): void {
    setOpen(false)
  }

  function applyVolume(data: unknown): void {
    const record = recordOf(data)
    volume = numberOf(record.volume, volume)
    muted = record.muted === true
  }

  function applyBrightness(data: unknown): void {
    const record = recordOf(data)
    brightness = numberOf(record.percent, brightness)
    brightnessAvailable = record.available === true
  }

  function applyBattery(data: unknown): void {
    const record = recordOf(data)
    batteryPercent = numberOf(record.percent, -1)
    batteryStatus = stringOf(record.status)
  }

  // Hyprland announces layout switches as "activelayout>>DEVICE,LAYOUT"; the
  // layout is everything after the last comma, since device names contain them.
  function applyKeyboardLayout(data: unknown): void {
    const record = recordOf(data)
    if (record.event !== 'activelayout' || typeof record.data !== 'string') {
      return
    }
    const separator = record.data.lastIndexOf(',')
    if (separator < 0) {
      return
    }
    keyboardLayout = record.data.slice(separator + 1)
  }

  function setVolume(percent: number): void {
    volume = percent
    void bus.call('system:setVolume', { volume: percent }, COMMAND_TIMEOUT_MS)
  }

  function toggleMute(): void {
    muted = !muted
    void bus.call('system:setMuted', { muted }, COMMAND_TIMEOUT_MS)
  }

  function setBrightness(percent: number): void {
    brightness = percent
    void bus.call('system:setBrightness', { percent }, COMMAND_TIMEOUT_MS)
  }

  function toggleDoNotDisturb(): void {
    doNotDisturb = !doNotDisturb
    bus.publish('config:set', { key: 'notifications.doNotDisturb', value: doNotDisturb })
  }

  function captureScreen(): void {
    close()
    void bus.call(
      'hypr:dispatch',
      { dispatcher: 'exec', arg: SCREENSHOT_COMMAND },
      COMMAND_TIMEOUT_MS,
    )
  }

  async function runPowerAction(action: string): Promise<void> {
    close()
    const reply = await bus.call('system:power', { action }, COMMAND_TIMEOUT_MS)
    const error = errorOf(reply)
    if (error !== '') {
      console.error(`quicksettings: ${action} failed:`, error)
    }
  }

  function toggleWifi(): void {
    void bus.call('network:enable', { enabled: !network.enabled }, 15000)
  }

  function toggleBluetooth(): void {
    void bus.call('bluetooth:power', { powered: !bluetooth.powered }, 15000)
  }

  function wifiTitle(): string {
    if (network.connected) {
      return network.ssid
    }
    return 'Wi-Fi'
  }

  function wifiSubtitle(): string {
    if (!network.available) {
      return 'No adapter'
    }
    if (!network.enabled) {
      return 'Off'
    }
    if (!network.connected) {
      return 'Not connected'
    }
    return signalLabel(network.signal)
  }

  function bluetoothSubtitle(): string {
    if (!bluetooth.available) {
      return 'No adapter'
    }
    if (!bluetooth.powered) {
      return 'Off'
    }
    if (connectedDevice !== undefined) {
      return connectedDevice.name
    }
    return 'On'
  }

  function bluetoothIcon(): string {
    if (connectedDevice === undefined) {
      return 'bluetooth'
    }
    return deviceIcon(connectedDevice.icon)
  }

  function keyboardSubtitle(): string {
    if (keyboardLayout === '') {
      return 'Default'
    }
    return keyboardLayout
  }

  function volumeIcon(): string {
    if (muted) {
      return 'volumeMuted'
    }
    return 'volume'
  }

  function batteryLabel(): string {
    if (batteryStatus === '') {
      return `${batteryPercent}%`
    }
    return `${batteryPercent}% · ${batteryStatus}`
  }

  function powerItemStyle(action: string): Style {
    if (action === 'poweroff') {
      return { color: colors.error }
    }
    return {}
  }

  function powerItemHover(action: string): Style {
    if (action === 'poweroff') {
      return { background: alpha(colors.error, 0.15) }
    }
    return { background: alpha(colors.content, 0.08) }
  }

  function networkStateOf(data: unknown): NetworkState {
    const record = recordOf(data)
    return {
      available: record.available === true,
      enabled: record.enabled === true,
      connected: record.connected === true,
      ssid: stringOf(record.ssid),
      signal: numberOf(record.signal, 0),
      secured: record.secured === true,
      device: stringOf(record.device),
      ipv4: stringOf(record.ipv4),
      gateway: stringOf(record.gateway),
      dns: stringListOf(record.dns),
    }
  }

  function networksOf(data: unknown): WifiNetwork[] {
    return arrayOf(data).map((record) => ({
      ssid: stringOf(record.ssid),
      signal: numberOf(record.signal, 0),
      secured: record.secured === true,
      active: record.active === true,
      saved: record.saved === true,
    }))
  }

  function bluetoothStateOf(data: unknown): BluetoothState {
    const record = recordOf(data)
    return {
      available: record.available === true,
      powered: record.powered === true,
      discovering: record.discovering === true,
      adapter: stringOf(record.adapter),
    }
  }

  function devicesOf(data: unknown): BluetoothDevice[] {
    return arrayOf(data).map((record) => ({
      mac: stringOf(record.mac),
      name: stringOf(record.name),
      connected: record.connected === true,
      paired: record.paired === true,
      trusted: record.trusted === true,
      icon: stringOf(record.icon),
      battery: numberOf(record.battery, -1),
    }))
  }

  function emptyNetworkState(): NetworkState {
    return {
      available: false,
      enabled: false,
      connected: false,
      ssid: '',
      signal: 0,
      secured: false,
      device: '',
      ipv4: '',
      gateway: '',
      dns: [],
    }
  }

  function emptyBluetoothState(): BluetoothState {
    return { available: false, powered: false, discovering: false, adapter: '' }
  }



  const POWER_ITEMS = [
    { action: 'lock', label: 'Lock' },
    { action: 'logout', label: 'Log out' },
    { action: 'suspend', label: 'Sleep' },
    { action: 'reboot', label: 'Restart' },
    { action: 'poweroff', label: 'Power off' },
  ]
</script>

<Surface {args} keyboard="on-demand" style={{ background: windowBackground }}>
  {#if open}
    <!-- Takes the click that lands anywhere but on the panel. -->
    <View
      input
      style={{ position: 'absolute', inset: 0, background: windowBackground }}
      onpress={close}
    ></View>

    <Presence
      in={[fly, { y: -10, duration: 160 }]}
      out={fade}
      style={{ position: 'absolute', top: offsetTop, right: offsetEnd, width: PANEL_WIDTH }}
    >
      <!-- A press that lands on the panel and on nothing in it must not fall
           through to the backdrop under it: a handler that takes it stops it. -->
      <View style={panelStyle} ondown={keepPress}>
        {#if page === 'wifi'}
          <WifiPage {bus} networkState={network} {networks} onBack={() => (page = 'main')} />
        {:else if page === 'bluetooth'}
          <BluetoothPage {bus} bluetoothState={bluetooth} {devices} onBack={() => (page = 'main')} />
        {:else}
          {@render mainPage()}
        {/if}
      </View>
    </Presence>
  {:else}
    <!-- An empty region is what keeps a closed tray click-through. -->
    <View input style={{ width: 0, height: 0 }}></View>
  {/if}
</Surface>

{#snippet mainPage()}
  <View style={{ gap: 8 }}>
    <View style={{ direction: 'row', gap: 8 }}>
      <Tile
        icon={signalIcon(network.signal, network.connected)}
        title={wifiTitle()}
        subtitle={wifiSubtitle()}
        active={network.enabled}
        disabled={!network.available}
        onActivate={toggleWifi}
        onOpen={() => (page = 'wifi')}
      />
      <View style={{ flex: 1, direction: 'row', gap: 8 }}>
        <Tile compact icon="camera" title="Screen capture" onActivate={captureScreen} />
        <Tile compact icon="moon" title="Do not disturb" active={doNotDisturb} onActivate={toggleDoNotDisturb} />
      </View>
    </View>

    <View style={{ direction: 'row', gap: 8 }}>
      <Tile
        icon={bluetoothIcon()}
        title="Bluetooth"
        subtitle={bluetoothSubtitle()}
        active={bluetooth.powered}
        disabled={!bluetooth.available}
        onActivate={toggleBluetooth}
        onOpen={() => (page = 'bluetooth')}
      />
      <Tile icon="cast" title="Cast screen" subtitle="Unavailable" disabled />
    </View>

    <View style={{ direction: 'row', gap: 8 }}>
      <Tile icon="share" title="Nearby share" subtitle="Unavailable" disabled />
      <Tile icon="keyboard" title="Keyboard" subtitle={keyboardSubtitle()} disabled />
    </View>
  </View>

  <View style={{ gap: 8 }}>
    <SliderRow icon={volumeIcon()} label="Volume" value={volume} onInput={setVolume} onIconClick={toggleMute} />
    <SliderRow
      icon="sun"
      label="Brightness"
      value={brightness}
      disabled={!brightnessAvailable}
      onInput={setBrightness}
    />
  </View>

  <View style={separatorStyle}></View>

  <View style={{ direction: 'row', align: 'center', gap: 4, paddingX: 2, paddingTop: 2 }}>
    <Tooltip text="Power options" side="top">
      <Button
        style={{
          direction: 'row',
          align: 'center',
          gap: 2,
          paddingX: 6,
          paddingY: 4,
          radius: 16,
          color: alpha(colors.content, 0.7),
          transition: 120,
        }}
        hover={{ background: alpha(colors.content, 0.1) }}
        onpress={() => (powerMenuOpen = !powerMenuOpen)}
      >
        <Icon name={iconName('power')} size={16} />
        <Icon name={iconName('chevronDown')} size={12} />
      </Button>
    </Tooltip>
    <View style={{ flex: 1 }}></View>
    {#if batteryPercent >= 0}
      <Text style={{ fontSize: 11, paddingX: 4, color: alpha(colors.content, 0.6) }}>{batteryLabel()}</Text>
    {/if}
  </View>

  {#if powerMenuOpen}
    <View style={{ gap: 2, padding: 4, radius: 16, background: alpha(colors.surface, 0.7) }}>
      {#each POWER_ITEMS as item (item.action)}
        <Button
          style={{ ...powerItemStyle(item.action), paddingX: 12, paddingY: 6, radius: 12, fontSize: 12 }}
          hover={powerItemHover(item.action)}
          onpress={() => void runPowerAction(item.action)}
        >
          <Text>{item.label}</Text>
        </Button>
      {/each}
    </View>
  {/if}
{/snippet}
