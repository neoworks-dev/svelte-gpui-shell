<script lang="ts">
  import { errorOf } from '../../lib/bus'
  import type { BusService } from '../../lib/bus'
  import { Button, Icon, Input, Scroll, Switch, Text, View } from '@neoworks/svelte-gpui/ui'
  import { alpha, colors } from '../../theme'
  import { iconName, signalIcon, signalLabel } from './icons'
  import {
    activeRow,
    buttonHover,
    buttonStyle,
    detailsStyle,
    emptyStyle,
    failureStyle,
    listHeight,
    pageHeaderStyle,
    pageTitleStyle,
    primaryButtonHover,
    primaryButtonStyle,
    roundButtonHover,
    roundButtonStyle,
    rowHover,
    rowNoteStyle,
    rowStyle,
    rowTitleStyle,
  } from './look'
  import type { NetworkState, WifiNetwork } from './types'
  import type { Style } from '@neoworks/svelte-gpui/ui'

  // The Wi-Fi detail page: radio toggle, scan, the network list, the password
  // prompt for a secured network with no saved secret, and the details of the
  // connection in use.

  const CONNECT_TIMEOUT_MS = 45000
  const COMMAND_TIMEOUT_MS = 30000

  type Mode = 'list' | 'password' | 'details'

  let {
    bus,
    networkState,
    networks,
    onBack,
  }: {
    bus: BusService
    networkState: NetworkState
    networks: WifiNetwork[]
    onBack: () => void
  } = $props()

  let mode = $state<Mode>('list')
  let selected = $state<WifiNetwork | null>(null)
  let password = $state('')
  let busy = $state(false)
  let failure = $state('')
  let peek = $state(false)

  const title = $derived(titleOf(mode, selected))

  function titleOf(current: Mode, network: WifiNetwork | null): string {
    if (current === 'list') {
      return 'Wi-Fi'
    }
    if (network === null) {
      return 'Wi-Fi'
    }
    return network.ssid
  }

  function backToList(): void {
    mode = 'list'
    selected = null
    password = ''
    failure = ''
  }

  function goBack(): void {
    if (mode === 'list') {
      onBack()
      return
    }
    backToList()
  }

  // A saved or open network connects straight away; a secured one the machine
  // has never joined needs its passphrase first.
  function choose(network: WifiNetwork): void {
    failure = ''
    selected = network
    if (network.active) {
      mode = 'details'
      return
    }
    if (network.secured && !network.saved) {
      password = ''
      mode = 'password'
      return
    }
    void connect(network.ssid, '')
  }

  async function connect(ssid: string, secret: string): Promise<void> {
    busy = true
    failure = ''
    const reply = await bus.call('network:connect', { ssid, password: secret }, CONNECT_TIMEOUT_MS)
    busy = false
    const error = errorOf(reply)
    if (error !== '') {
      failure = error
      return
    }
    backToList()
  }

  function submitPassword(): void {
    if (selected === null || password === '') {
      return
    }
    void connect(selected.ssid, password)
  }

  async function runCommand(type: string, data: unknown): Promise<void> {
    busy = true
    failure = ''
    const reply = await bus.call(type, data, COMMAND_TIMEOUT_MS)
    busy = false
    failure = errorOf(reply)
  }

  function toggleRadio(wanted: boolean): void {
    void runCommand('network:enable', { enabled: wanted })
  }

  function rescan(): void {
    void runCommand('network:scan', {})
  }

  function disconnect(): void {
    void runCommand('network:disconnect', {})
    backToList()
  }

  function forget(): void {
    if (selected === null) {
      return
    }
    void runCommand('network:forget', { ssid: selected.ssid })
    backToList()
  }

  function rowStyleOf(network: WifiNetwork): Style {
    if (network.active) {
      return { ...rowStyle, ...activeRow }
    }
    return rowStyle
  }

  function peekIcon(isPeeking: boolean): string {
    if (isPeeking) {
      return 'view-conceal-symbolic'
    }
    return 'view-reveal-symbolic'
  }

  function securityLabel(secured: boolean): string {
    if (secured) {
      return 'Protected'
    }
    return 'Open'
  }

  function emptyMessage(state: NetworkState, count: number): string {
    if (!state.available) {
      return 'No Wi-Fi device on this machine.'
    }
    if (!state.enabled) {
      return 'Wi-Fi is off.'
    }
    if (count === 0) {
      return 'No networks found.'
    }
    return ''
  }


</script>

<View style={{ gap: 12 }}>
  <View style={pageHeaderStyle}>
    <!-- TODO(host: popups) the "Back", "Scan for networks" and "Wi-Fi enabled" tooltips. -->
    <Button style={roundButtonStyle} hover={roundButtonHover} onpress={goBack}>
      <Icon name={iconName('chevronLeft')} size={18} />
    </Button>
    <Text style={pageTitleStyle}>{title}</Text>
    {#if mode === 'list'}
      <Button
        style={roundButtonStyle}
        hover={roundButtonHover}
        disabled={busy || !networkState.enabled}
        disabledStyle={{ opacity: 0.4 }}
        onpress={rescan}
      >
        <Icon name={iconName('refresh')} size={16} />
      </Button>
      <Switch
        checked={networkState.enabled}
        disabled={busy || !networkState.available}
        trackStyle={{ background: alpha(colors.content, 0.2) }}
        checkedStyle={{ background: colors.info }}
        onchange={toggleRadio}
      />
    {/if}
  </View>

  {#if failure !== ''}
    <Text style={failureStyle}>{failure}</Text>
  {/if}

  {#if mode === 'list'}
    {@const empty = emptyMessage(networkState, networks.length)}
    {#if empty !== ''}
      <Text style={emptyStyle}>{empty}</Text>
    {:else}
      <Scroll style={{ maxHeight: listHeight }}>
        <View style={{ gap: 2 }}>
          {#each networks as network (network.ssid)}
            <Button
              style={rowStyleOf(network)}
              hover={rowHover}
              disabled={busy}
              onpress={() => choose(network)}
            >
              <Icon name={iconName(signalIcon(network.signal, true))} size={20} />
              <View style={{ flex: 1, minWidth: 0, justify: 'center' }}>
                <Text style={rowTitleStyle}>{network.ssid}</Text>
                {#if network.active}
                  <Text style={{ ...rowNoteStyle, color: colors.info }}>Connected</Text>
                {:else if network.saved}
                  <Text style={rowNoteStyle}>Saved</Text>
                {/if}
              </View>
              {#if network.secured}
                <Icon name={iconName('lock')} size={13} />
              {/if}
              <Icon name={iconName('chevronRight')} size={14} />
            </Button>
          {/each}
        </View>
      </Scroll>
    {/if}
  {/if}

  {#if mode === 'password' && selected !== null}
    <View style={{ gap: 12 }}>
      <Text style={{ fontSize: 11, color: alpha(colors.content, 0.6) }}>
        Enter the password for {selected.ssid}.
      </Text>
      <!-- A password field only takes a keystroke while the layer surface holds
           the keyboard, which the tray's node asks for with keyboard: ondemand. -->
      <View style={{ direction: 'row', align: 'center', gap: 4 }}>
        <Input
          bind:value={password}
          placeholder="Password"
          secure={!peek}
          disabled={busy}
          style={{
            flex: 1,
            paddingX: 10,
            paddingY: 6,
            radius: 6,
            fontSize: 12,
            background: colors.surface,
          }}
          onsubmit={submitPassword}
        />
        <Button style={roundButtonStyle} hover={roundButtonHover} onpress={() => (peek = !peek)}>
          <Icon name={peekIcon(peek)} size={16} />
        </Button>
      </View>
      <View style={{ direction: 'row', gap: 8, justify: 'end' }}>
        <Button style={buttonStyle} hover={buttonHover} onpress={backToList}>
          <Text>Cancel</Text>
        </Button>
        <Button
          style={{ ...buttonStyle, ...primaryButtonStyle }}
          hover={primaryButtonHover}
          disabled={busy || password === ''}
          disabledStyle={{ opacity: 0.4 }}
          onpress={submitPassword}
        >
          <Text>Connect</Text>
        </Button>
      </View>
    </View>
  {/if}

  {#if mode === 'details' && selected !== null}
    <View style={detailsStyle}>
      {@render detail('Signal', `${signalLabel(networkState.signal)} · ${networkState.signal}%`)}
      {@render detail('Security', securityLabel(networkState.secured))}
      {@render detail('IPv4', networkState.ipv4)}
      {@render detail('Gateway', networkState.gateway)}
      {@render detail('DNS', networkState.dns.join(', '))}
      {@render detail('Interface', networkState.device)}
    </View>
    <View style={{ direction: 'row', gap: 8, justify: 'end' }}>
      <Button style={buttonStyle} hover={buttonHover} disabled={busy} onpress={forget}>
        <Text>Forget</Text>
      </Button>
      <Button style={buttonStyle} hover={buttonHover} disabled={busy} onpress={disconnect}>
        <Text>Disconnect</Text>
      </Button>
    </View>
  {/if}
</View>

{#snippet detail(key: string, value: string)}
  <View style={{ direction: 'row', gap: 12 }}>
    <Text style={{ flex: 1, fontSize: 11, color: alpha(colors.content, 0.5) }}>{key}</Text>
    <Text style={{ fontSize: 11, truncate: true, maxWidth: 200 }}>{value}</Text>
  </View>
{/snippet}
