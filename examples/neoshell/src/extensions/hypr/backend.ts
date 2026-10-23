import type { Plugin } from '@neoworks/extension-system'
import { requireService } from '../../lib/services.js'
import { registerFunction, RetainedTopics } from '../../lib/bus.js'
import type { BusService } from '../../lib/bus.js'
import { environment, fileExists, modifiedAt, readDirectory, runtimeDirectory } from '../../system/fs.js'
import { request, streamLines } from '../../system/socket.js'

// hypr: the compositor bridge. Window and workspace state is what the bar,
// dock and desktop all consume. It is published as retained topics with no
// Hyprland types leaking into the names, so a different compositor bridge can
// provide the same contract:
//
//   hypr.windows / hypr.workspaces / hypr.activeworkspace /
//   hypr.activewindow / hypr.monitors        retained query snapshots
//   hypr.event                               raw compositor events, live
//   hypr:dispatch {dispatcher, arg}          → {ok} | {error}
//   hypr:keyword  {name, value}              → {ok} | {error}
//   hypr:layerrule {namespace, blur?, blurPopups?, ignoreAlpha?}
//                                            → {ok} | {error}
//   hypr:request  {command}                  → {reply} | {error}
//
// It also provides the in-kernel "hypr" service for other extensions.

interface HyprPaths {
  requestSocket: string
  eventSocket: string
}

interface HyprConfig {
  requestSocket?: string
  eventSocket?: string
}

const REQUEST_TIMEOUT_MS = 3000
const REFRESH_DEBOUNCE_MS = 50
const RECONNECT_DELAY_MS = 1000

const hyprExtension: Plugin.Object<HyprConfig | undefined> = {
  name: 'hypr',
  inject: ['bus'],
  async apply(context, config) {
    const bus = requireService<BusService>(context, 'bus')
    const paths = await resolvePaths(config)
    const client = new HyprClient(paths)
    const publisher = new HyprPublisher(bus, client)
    context.effect(() => publisher.start())
    context.effect(() =>
      streamLines(paths.eventSocket, (line) => publisher.handleEvent(line), RECONNECT_DELAY_MS),
    )
    registerFunction(context, bus, 'hypr:dispatch', (data) => runDispatch(client, data))
    registerFunction(context, bus, 'hypr:keyword', (data) => runKeyword(client, data))
    registerFunction(context, bus, 'hypr:layerrule', (data) => runLayerRule(client, data))
    registerFunction(context, bus, 'hypr:request', (data) => runRequest(client, data))
    context.provide('hypr', client)
  },
}

export default hyprExtension

async function resolvePaths(config: HyprConfig | undefined): Promise<HyprPaths> {
  if (config !== undefined && config.requestSocket !== undefined && config.eventSocket !== undefined) {
    return { requestSocket: config.requestSocket, eventSocket: config.eventSocket }
  }
  const signature = (await environment()).HYPRLAND_INSTANCE_SIGNATURE
  const base = await resolveInstanceDir(`${await runtimeDirectory()}/hypr`, signature)
  return { requestSocket: `${base}/.socket.sock`, eventSocket: `${base}/.socket2.sock` }
}

// resolveInstanceDir prefers the signature, but a shell started from an older
// session's environment inherits one whose socket is long gone — dead
// signatures fall back to the newest instance that still has one.
export async function resolveInstanceDir(root: string, signature: string | undefined): Promise<string> {
  if (signature !== undefined && signature !== '' && (await hasRequestSocket(`${root}/${signature}`))) {
    return `${root}/${signature}`
  }
  const newest = await newestInstance(root)
  if (newest === null) {
    throw new Error(`hypr: no live compositor instance under ${root}`)
  }
  console.warn(`hypr: HYPRLAND_INSTANCE_SIGNATURE is stale, using ${newest}`)
  return newest
}

async function newestInstance(root: string): Promise<string | null> {
  const live = await liveInstances(root)
  if (live.length === 0) {
    return null
  }
  const modified = new Map<string, number>()
  for (const dir of live) {
    modified.set(dir, await modifiedAt(dir))
  }
  live.sort((left, right) => (modified.get(right) as number) - (modified.get(left) as number))
  return live[0]
}

async function liveInstances(root: string): Promise<string[]> {
  const live: string[] = []
  for (const entry of await readDirectory(root)) {
    if (await hasRequestSocket(`${root}/${entry}`)) {
      live.push(`${root}/${entry}`)
    }
  }
  return live
}

function hasRequestSocket(dir: string): Promise<boolean> {
  return fileExists(`${dir}/.socket.sock`)
}

// HyprClient speaks Hyprland's request IPC: one command per connection,
// Hyprland writes the reply and closes, so reading to EOF is the completion
// signal.
export class HyprClient {
  private readonly paths: HyprPaths

  constructor(paths: HyprPaths) {
    this.paths = paths
  }

  request(command: string): Promise<string> {
    return request(this.paths.requestSocket, command, REQUEST_TIMEOUT_MS)
  }

  async requestJson<Result>(command: string): Promise<Result> {
    const raw = await this.request(`j/${command}`)
    return JSON.parse(raw) as Result
  }

  // Hyprland ≥ 0.56 reads `dispatch` as a Lua expression returning a
  // dispatcher; the classic `name arg` spelling no longer parses.
  dispatch(dispatcher: string, arg: string): Promise<string> {
    return this.request(`dispatch ${luaDispatcher(dispatcher, arg)}`)
  }

  keyword(name: string, value: string): Promise<string> {
    return this.request(`keyword ${name} ${value}`)
  }

  // Hyprland ≥ 0.56 takes rules as Lua through `eval`; `keyword layerrule`
  // is refused by the non-legacy config parser.
  eval(lua: string): Promise<string> {
    return this.request(`eval ${lua}`)
  }
}

// HyprPublisher keeps the retained state topics current: a full refresh on
// start, then a debounced refresh on every compositor event. Per-query
// failures (e.g. no active window) skip that topic and keep the rest.
class HyprPublisher {
  private readonly bus: BusService
  private readonly client: HyprClient
  private readonly retained: RetainedTopics
  private refreshTimer: ReturnType<typeof setTimeout> | null = null

  constructor(bus: BusService, client: HyprClient) {
    this.bus = bus
    this.client = client
    this.retained = new RetainedTopics(bus)
  }

  start(): () => void {
    void this.refreshAll()
    return () => {
      this.cancelScheduledRefresh()
      this.retained.withdrawAll()
    }
  }

  handleEvent(line: string): void {
    const separatorIndex = line.indexOf('>>')
    if (separatorIndex < 0) {
      return
    }
    this.bus.publish('hypr.event', {
      event: line.slice(0, separatorIndex),
      data: line.slice(separatorIndex + 2),
    })
    this.scheduleRefresh()
  }

  private scheduleRefresh(): void {
    if (this.refreshTimer !== null) {
      return
    }
    this.refreshTimer = setTimeout(() => {
      this.refreshTimer = null
      void this.refreshAll()
    }, REFRESH_DEBOUNCE_MS)
  }

  private cancelScheduledRefresh(): void {
    if (this.refreshTimer !== null) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = null
    }
  }

  private async refreshAll(): Promise<void> {
    await this.publishQuery('hypr.windows', 'clients')
    await this.publishQuery('hypr.workspaces', 'workspaces')
    await this.publishQuery('hypr.activeworkspace', 'activeworkspace')
    await this.publishQuery('hypr.activewindow', 'activewindow')
    await this.publishQuery('hypr.monitors', 'monitors')
  }

  private async publishQuery(topic: string, command: string): Promise<void> {
    let data: unknown
    try {
      data = await this.client.requestJson(command)
    } catch {
      return
    }
    this.retained.set(topic, data)
  }
}

// luaDispatcher maps the dispatcher names the views use onto the Lua API.
function luaDispatcher(dispatcher: string, arg: string): string {
  if (dispatcher === 'focuswindow') {
    return `hl.dsp.focus({ window = ${luaString(arg)} })`
  }
  if (dispatcher === 'workspace') {
    return `hl.dsp.focus({ workspace = ${luaString(arg)} })`
  }
  if (dispatcher === 'exec') {
    return `hl.dsp.exec_cmd(${luaString(arg)})`
  }
  if (dispatcher === 'exit') {
    return 'hl.dsp.exit()'
  }
  throw new Error(`hypr: no Lua spelling for dispatcher ${dispatcher}`)
}

function luaString(value: string): string {
  return JSON.stringify(value)
}

async function runDispatch(client: HyprClient, data: unknown): Promise<unknown> {
  const args = data as { dispatcher?: string; arg?: string }
  if (typeof args.dispatcher !== 'string' || args.dispatcher === '') {
    return { error: 'dispatcher is required' }
  }
  let reply: string
  try {
    reply = await client.dispatch(args.dispatcher, stringOrEmpty(args.arg))
  } catch (error) {
    return { error: String(error) }
  }
  if (reply.startsWith('error')) {
    return { error: reply }
  }
  return { ok: true }
}

async function runKeyword(client: HyprClient, data: unknown): Promise<unknown> {
  const args = data as { name?: string; value?: string }
  if (typeof args.name !== 'string' || args.name === '') {
    return { error: 'name is required' }
  }
  await client.keyword(args.name, stringOrEmpty(args.value))
  return { ok: true }
}

interface LayerRuleArgs {
  namespace?: string
  blur?: boolean
  blurPopups?: boolean
  ignoreAlpha?: number
}

async function runLayerRule(client: HyprClient, data: unknown): Promise<unknown> {
  const args = data as LayerRuleArgs
  if (typeof args.namespace !== 'string' || args.namespace === '') {
    return { error: 'namespace is required' }
  }
  const fields = [`match = { namespace = ${luaString(`^${escapeRegex(args.namespace)}$`)} }`]
  if (args.blur === true) {
    fields.push('blur = true')
  }
  if (args.blurPopups === true) {
    fields.push('blur_popups = true')
  }
  if (typeof args.ignoreAlpha === 'number') {
    fields.push(`ignore_alpha = ${args.ignoreAlpha}`)
  }
  let reply: string
  try {
    reply = await client.eval(`hl.layer_rule({ ${fields.join(', ')} })`)
  } catch (error) {
    return { error: String(error) }
  }
  if (reply.startsWith('error')) {
    return { error: reply }
  }
  return { ok: true }
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

async function runRequest(client: HyprClient, data: unknown): Promise<unknown> {
  const args = data as { command?: string }
  if (typeof args.command !== 'string' || args.command === '') {
    return { error: 'command is required' }
  }
  return { reply: await client.request(args.command) }
}

function numberOr(value: number | undefined, fallback: number): number {
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
    return value
  }
  return fallback
}

function stringOrEmpty(value: string | undefined): string {
  if (value === undefined) {
    return ''
  }
  return value
}
