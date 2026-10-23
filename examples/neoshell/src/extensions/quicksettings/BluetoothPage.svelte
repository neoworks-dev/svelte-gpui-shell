<script lang="ts">
  import { errorOf } from '../../lib/bus'
  import type { BusService } from '../../lib/bus'
  import { Button, Icon, Scroll, Spinner, Switch, Text, Tooltip, View } from '@neoworks/svelte-gpui/ui'
  import type { Style } from '@neoworks/svelte-gpui/ui'
  import { alpha, colors } from '../../theme'
  import { deviceIcon, iconName } from './icons'
  import {
    activeRow,
    emptyStyle,
    failureStyle,
    listHeight,
    pageHeaderStyle,
    pageTitleStyle,
    roundButtonHover,
    roundButtonStyle,
    rowHover,
    rowNoteStyle,
    rowStyle,
    rowTitleStyle,
    sectionStyle,
  } from './look'
  import type { BluetoothDevice, BluetoothState } from './types'

  // The Bluetooth detail page: adapter toggle, discovery, and the device list
  // split into paired devices and everything else discovery has turned up.

  const COMMAND_TIMEOUT_MS = 60000

  let {
    bus,
    bluetoothState,
    devices,
    onBack,
  }: {
    bus: BusService
    bluetoothState: BluetoothState
    devices: BluetoothDevice[]
    onBack: () => void
  } = $props()

  let busy = $state(false)
  let failure = $state('')
  let pendingMac = $state('')

  const paired = $derived(devices.filter((device) => device.paired))
  const discovered = $derived(devices.filter((device) => !device.paired))
  const empty = $derived(emptyMessage(bluetoothState))

  async function runCommand(type: string, data: unknown, mac: string): Promise<void> {
    busy = true
    pendingMac = mac
    failure = ''
    const reply = await bus.call(type, data, COMMAND_TIMEOUT_MS)
    busy = false
    pendingMac = ''
    failure = errorOf(reply)
  }

  function togglePower(wanted: boolean): void {
    void runCommand('bluetooth:power', { powered: wanted }, '')
  }

  function scan(): void {
    void runCommand('bluetooth:scan', {}, '')
  }

  // A paired device toggles its connection; an unpaired one is paired,
  // trusted, and connected in one step by the backend.
  function choose(device: BluetoothDevice): void {
    if (!device.paired) {
      void runCommand('bluetooth:pair', { mac: device.mac }, device.mac)
      return
    }
    if (device.connected) {
      void runCommand('bluetooth:disconnect', { mac: device.mac }, device.mac)
      return
    }
    void runCommand('bluetooth:connect', { mac: device.mac }, device.mac)
  }

  function forget(device: BluetoothDevice): void {
    void runCommand('bluetooth:forget', { mac: device.mac }, device.mac)
  }

  function statusOf(device: BluetoothDevice): string {
    if (device.connected && device.battery >= 0) {
      return `Connected · ${device.battery}%`
    }
    if (device.connected) {
      return 'Connected'
    }
    if (device.paired) {
      return 'Paired'
    }
    return ''
  }

  function rowStyleOf(device: BluetoothDevice): Style {
    if (device.connected) {
      return { ...rowStyle, ...activeRow }
    }
    return rowStyle
  }

  function emptyMessage(state: BluetoothState): string {
    if (!state.available) {
      return 'No Bluetooth adapter on this machine.'
    }
    if (!state.powered) {
      return 'Bluetooth is off.'
    }
    return ''
  }


</script>

<View style={{ gap: 12 }}>
  <View style={pageHeaderStyle}>
    <Tooltip text="Back">
      <Button style={roundButtonStyle} hover={roundButtonHover} onpress={onBack}>
        <Icon name={iconName('chevronLeft')} size={18} />
      </Button>
    </Tooltip>
    <Text style={pageTitleStyle}>Bluetooth</Text>
    <Tooltip text="Scan for devices">
      <Button
        style={roundButtonStyle}
        hover={roundButtonHover}
        disabled={busy || !bluetoothState.powered}
        disabledStyle={{ opacity: 0.4 }}
        onpress={scan}
      >
        <Icon name={iconName('refresh')} size={16} />
      </Button>
    </Tooltip>
    <Tooltip text="Bluetooth enabled">
      <Switch
        checked={bluetoothState.powered}
        disabled={busy || !bluetoothState.available}
        trackStyle={{ background: alpha(colors.content, 0.2) }}
        checkedStyle={{ background: colors.info }}
        onchange={togglePower}
      />
    </Tooltip>
  </View>

  {#if failure !== ''}
    <Text style={failureStyle}>{failure}</Text>
  {/if}

  {#if empty !== ''}
    <Text style={emptyStyle}>{empty}</Text>
  {:else}
    <Scroll style={{ maxHeight: listHeight }}>
      <View style={{ gap: 12 }}>
        {#if paired.length > 0}
          <View style={{ gap: 2 }}>
            <Text style={sectionStyle}>YOUR DEVICES</Text>
            {#each paired as device (device.mac)}
              <View style={{ ...rowStyleOf(device), overflow: 'hidden', paddingX: 0, paddingY: 0, gap: 0 }}>
                <Button
                  style={{ flex: 1, minWidth: 0, direction: 'row', align: 'center', gap: 12, paddingX: 10, paddingY: 8 }}
                  disabled={busy}
                  onpress={() => choose(device)}
                >
                  <Icon name={iconName(deviceIcon(device.icon))} size={20} />
                  <View style={{ flex: 1, minWidth: 0, justify: 'center' }}>
                    <Text style={rowTitleStyle}>{device.name}</Text>
                    <Text style={rowNoteStyle}>{statusOf(device)}</Text>
                  </View>
                  {#if pendingMac === device.mac}
                    <Spinner size={16} />
                  {/if}
                </Button>
                <Button
                  style={{ paddingX: 6, fontSize: 10, color: alpha(colors.content, 0.4), transition: 120 }}
                  hover={{ color: colors.content }}
                  disabled={busy}
                  onpress={() => forget(device)}
                >
                  <Text>Forget</Text>
                </Button>
              </View>
            {/each}
          </View>
        {/if}

        <View style={{ gap: 2 }}>
          <Text style={sectionStyle}>NEARBY</Text>
          {#if discovered.length === 0}
            <Text style={{ ...rowNoteStyle, paddingX: 10 }}>Nothing found yet — start a scan.</Text>
          {/if}
          {#each discovered as device (device.mac)}
            <Button style={rowStyle} hover={rowHover} disabled={busy} onpress={() => choose(device)}>
              <Icon name={iconName(deviceIcon(device.icon))} size={20} />
              <Text style={{ ...rowTitleStyle, flex: 1, minWidth: 0 }}>{device.name}</Text>
              {#if pendingMac === device.mac}
                <Spinner size={16} />
              {:else}
                <Text style={rowNoteStyle}>Pair</Text>
              {/if}
            </Button>
          {/each}
        </View>
      </View>
    </Scroll>
  {/if}
</View>
