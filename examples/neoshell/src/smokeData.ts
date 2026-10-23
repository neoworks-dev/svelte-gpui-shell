import type { Bus } from './host/bus.js'
import type { ViewRegistry } from './host/plugins/views.js'
import SmokeCard from './SmokeCard.svelte'

// The state the smoke harness's views read, as the backends would retain it.
// The numbers are chosen to reach the ends of what a view draws: a flat day, a
// zero index and an over-scale one, a full and an empty battery.

export function publishSmokeData(bus: Bus, registry: ViewRegistry): void {
  answerCommands(bus)
  registerCard(bus, registry)
  publishWeather(bus)
  publishAirQuality(bus)
  publishSystem(bus)
  publishNetwork(bus)
  // The tray draws only while it is open, and a retained command is what a late subscriber still hears.
  bus.retain('quicksettings:open', {})
}

// Every command a view calls is answered, so no call waits out its timeout.
function answerCommands(bus: Bus): void {
  bus.subscribe('*', (message) => {
    if (message.replyTo !== undefined && !message.type.startsWith('reply.')) {
      bus.publish(message.replyTo, { ok: true })
    }
  })
}

// The gallery previews whatever the registry holds for a catalog entry's type,
// so a catalog with no provider behind it would build the empty branch only.
function registerCard(bus: Bus, registry: ViewRegistry): void {
  registry.register('smoke.card', SmokeCard)
  bus.retain('widgets.catalog', {
    widgets: [
      {
        type: 'smoke.card',
        name: 'Smoke card',
        category: 'Smoke',
        description: 'a widget to preview',
        sizes: ['small'],
        defaultSize: 'small',
      },
    ],
  })
}

function publishWeather(bus: Bus): void {
  for (const size of ['small', 'medium', 'large']) {
    bus.retain(`weather.current/smoke.weather.${size}`, {
      place: 'Smoke',
      temperature: 17,
      unit: '°C',
      code: 61,
      description: 'Light rain',
      isDay: true,
      high: 21,
      low: 11,
      hours: [
        { label: 'Now', temperature: 17, code: 61, isDay: true },
        { label: '15', temperature: 18, code: 3, isDay: true },
        { label: '16', temperature: 18, code: 0, isDay: true },
        { label: '17', temperature: 16, code: 95, isDay: false },
      ],
      days: [
        { label: 'Today', code: 61, high: 21, low: 11 },
        { label: 'Tue', code: 0, high: 24, low: 13 },
        { label: 'Wed', code: 71, high: 15, low: 15 },
        { label: 'Thu', code: 95, high: 19, low: 9 },
      ],
    })
  }
}

function publishAirQuality(bus: Bus): void {
  bus.retain('airquality.current', {
    index: 42,
    scale: 'european',
    category: 'Fair',
    max: 100,
    place: 'Smoke',
    updatedAt: Date.now(),
    hours: [
      { label: 'Now', index: 42 },
      { label: '15', index: 88 },
      { label: '16', index: 0 },
      { label: '17', index: 140 },
    ],
    pollutants: [
      { label: 'PM2.5', value: 8, unit: 'µg/m³', share: 0.53 },
      { label: 'PM10', value: 61, unit: 'µg/m³', share: 1 },
      { label: 'NO₂', value: 1, unit: 'µg/m³', share: 0.04 },
    ],
  })
}

function publishSystem(bus: Bus): void {
  bus.retain('hypr.monitors', [{ width: 1920, height: 1080 }])
  bus.retain('system.volume', { volume: 40, muted: false })
  bus.retain('system.brightness', { percent: 60, available: true })
  bus.retain('system.battery', { percent: 80, charging: false, status: 'Discharging' })
  bus.retain('media.player', {
    status: 'Playing',
    title: 'Smoke track',
    artist: 'Smoke artist',
    album: '',
    artUrl: '',
    length: 180,
    shuffle: false,
  })
}

function publishNetwork(bus: Bus): void {
  bus.retain('network.state', {
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
  })
  bus.retain('bluetooth.state', { available: true, powered: true, discovering: false, adapter: 'smoke' })
  bus.retain('bluetooth.devices', [
    { mac: '00:00:00:00:00:01', name: 'paired', connected: true, paired: true, icon: 'audio-headset', battery: 80 },
    { mac: '00:00:00:00:00:02', name: 'nearby', connected: false, paired: false, icon: '', battery: -1 },
  ])
}
