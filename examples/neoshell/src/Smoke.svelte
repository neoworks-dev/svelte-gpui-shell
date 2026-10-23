<script lang="ts">
  import { Scroll, Text, View } from '@neoworks/svelte-gpui/ui'
  import type { BusService } from './lib/bus'
  import type { ViewRegistry } from './host/plugins/views'
  import AirQuality from './extensions/airquality/AirQuality.svelte'
  import BluetoothPage from './extensions/quicksettings/BluetoothPage.svelte'
  import Panel from './extensions/quicksettings/Panel.svelte'
  import WifiPage from './extensions/quicksettings/WifiPage.svelte'
  import Weather from './extensions/weather/Weather.svelte'
  import Battery from './extensions/neoshell/Battery.svelte'
  import Clock from './extensions/neoshell/Clock.svelte'
  import Desktop from './extensions/neoshell/Desktop.svelte'
  import Dock from './extensions/neoshell/Dock.svelte'
  import Notch from './extensions/neoshell/Notch.svelte'
  import Topbar from './extensions/neoshell/Topbar.svelte'
  import Wallpaper from './extensions/neoshell/Wallpaper.svelte'
  import WidgetGallery from './extensions/neoshell/WidgetGallery.svelte'
  import { MAX_UNIT_PX, sizePx, spanOf } from './extensions/neoshell/freeform'
  import { PANEL_WIDTH, panelStyle } from './extensions/quicksettings/look'
  import type { BluetoothState, NetworkState } from './extensions/quicksettings/types'

  // The smoke harness's markup: the views that need state of their own, built
  // once in a plain window. A page of the tray is reached by clicking, and its
  // state is the panel's own, so each is mounted again here, directly, with
  // the state it would be handed.

  let { bus, registry }: { bus: BusService; registry: ViewRegistry } = $props()

  const networkState: NetworkState = {
    available: true,
    enabled: true,
    connected: true,
    ssid: 'smoke',
    signal: 72,
    secured: true,
    device: 'wlan0',
    ipv4: '10.0.0.2/24',
    gateway: '10.0.0.1',
    dns: ['10.0.0.1'],
  }

  const bluetoothState: BluetoothState = {
    available: true,
    powered: true,
    discovering: false,
    adapter: 'smoke',
  }

  const networks = [
    { ssid: 'smoke', signal: 72, secured: true, active: true, saved: true },
    { ssid: 'other', signal: 20, secured: false, active: false, saved: false },
  ]

  const devices = [
    {
      mac: '00:00:00:00:00:01',
      name: 'paired device',
      connected: true,
      paired: true,
      trusted: true,
      icon: 'audio-headset',
      battery: 80,
    },
    {
      mac: '00:00:00:00:00:02',
      name: 'nearby device',
      connected: false,
      paired: false,
      trusted: false,
      icon: '',
      battery: -1,
    },
  ]

  // A widget card lays itself out from the size it is given, so every size is
  // built rather than only whichever one a default would have picked.
  const CARD_SIZES = ['small', 'medium', 'large']

  const SHELL_WIDTH = 900
  const SHELL_HEIGHT = 520
  const BAR_ARGS = { height: 30, centerGap: 320 }
  const DESKTOP_ARGS = {
    widgets: [
      { id: 'weather', type: 'weather.card', size: 'small' },
      { id: 'airquality', type: 'airquality.card', size: 'small' },
    ],
  }

  function boxFor(size: string) {
    const box = sizePx(MAX_UNIT_PX, spanOf(size))
    return { size, width: box.width, height: box.height }
  }
</script>

<Scroll style={{ flex: 1, background: '#101012' }}>
  <View style={{ direction: 'row', wrap: true, gap: 16, padding: 16, align: 'start' }}>
    <View style={{ width: SHELL_WIDTH, height: SHELL_HEIGHT, shrink: 0, background: '#101012' }}>
      <Wallpaper {bus} {registry} args={{}} id="smoke.wallpaper" />
      <Desktop {bus} {registry} args={DESKTOP_ARGS} id="smoke.desktop" />
      <View style={{ position: 'absolute', inset: 0 }}>
        <Topbar {bus} {registry} args={BAR_ARGS} id="smoke.bar">
          <Battery {bus} {registry} args={{}} id="smoke.battery" />
          <Clock {bus} {registry} args={{}} id="smoke.clock" />
        </Topbar>
      </View>
      <View style={{ position: 'absolute', top: 0, left: 0, right: 0, align: 'center' }}>
        <Notch {bus} {registry} args={{}} id="smoke.notch" />
      </View>
      <View style={{ position: 'absolute', bottom: 0, left: 0, right: 0 }}>
        <Dock {bus} {registry} args={{ height: 96 }} id="smoke.dock" />
      </View>
    </View>
    <View style={{ width: SHELL_WIDTH, height: 300, shrink: 0, background: '#101012' }}>
      <WidgetGallery {bus} {registry} args={{}} id="smoke.gallery" />
    </View>

    <View style={{ width: PANEL_WIDTH + 40, height: 720, shrink: 0 }}>
      <Panel {bus} {registry} args={{ offsetTop: 0, offsetEnd: 0 }} id="smoke.panel" />
    </View>
    <View style={{ ...panelStyle, width: PANEL_WIDTH, shrink: 0 }}>
      <WifiPage {bus} {networkState} {networks} onBack={() => {}} />
    </View>
    <View style={{ ...panelStyle, width: PANEL_WIDTH, shrink: 0 }}>
      <BluetoothPage {bus} {bluetoothState} {devices} onBack={() => {}} />
    </View>

    {#each CARD_SIZES as size (size)}
      {@const box = boxFor(size)}
      <View style={{ gap: 12, shrink: 0 }}>
        <View style={{ width: box.width, height: box.height }}>
          <Weather {bus} {registry} args={box} id={`smoke.weather.${size}`} />
        </View>
        <View style={{ width: box.width, height: box.height }}>
          <AirQuality {bus} {registry} args={box} id={`smoke.airquality.${size}`} />
        </View>
      </View>
    {/each}
  </View>
</Scroll>
