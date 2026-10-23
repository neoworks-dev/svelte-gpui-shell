# The window

An app has a window of its own. It opens it with options, reads what the window is like, and asks
things of it, all through `window` from `@neoworks/svelte-gpui`:

```ts
import { window } from "@neoworks/svelte-gpui";
```

This is not the browser's `window`: there is no DOM, and the only things on it are the ones
listed here. `window.innerWidth` and `<svelte:window onresize>` still work as on the web.

## Opening options

`start` takes the window's options as `options.window`, and the host applies them when it
opens the window:

```ts
start(App, {}, {
  window: { title: "My app", width: 1100, height: 760, minWidth: 720, minHeight: 480 },
});
```

| Option | Meaning |
|---|---|
| `title` | The title the window manager shows. |
| `width`, `height` | The size of the content in logical pixels. The default is 668 by 821. |
| `minWidth`, `minHeight` | The smallest size the user can drag the window to. It can only be set here. |

The host's `--width` and `--height` flags override the size, so one build can be tried at
several sizes. The size is what the app asks for. A tiling window manager decides the real
size of a window it tiles, and the app learns it from `window.size`.

## What the window is like

These are reactive. A component or `$effect` that reads one runs again when it changes, and
they are kept current as the user resizes, focuses or maximizes the window.

| | |
|---|---|
| `window.size` | `{ width, height }`, the content's size in logical pixels. |
| `window.scale` | Device pixels per logical pixel. 2 on a high-density display. |
| `window.focused` | Whether the window has the keyboard focus. |
| `window.maximized` | Whether the window is maximized. A tiling window manager may report `true` for a tiled window. |
| `window.fullscreen` | Whether the window is fullscreen. |

They hold their defaults (size 0 by 0, scale 1, focused) until the window has drawn its first
frame, which is before the user can see anything.

```svelte
<script lang="ts">
  import { window } from "@neoworks/svelte-gpui";
  import { Text } from "@neoworks/svelte-gpui/ui";
  const narrow = $derived(window.size.width < 700);
</script>

<Text style={{ opacity: window.focused ? 1 : 0.6 }}>{narrow ? "Compact" : "Wide"} layout</Text>
```

## Asking things of the window

| | |
|---|---|
| `window.setTitle(title)` | Changes the title. |
| `window.minimize()` | |
| `window.maximize()`, `window.unmaximize()`, `window.toggleMaximize()` | |
| `window.setFullscreen(on)`, `window.toggleFullscreen()` | |
| `window.close()` | Closes the window, as the close button does: the handlers from `onCloseRequested` are asked first. |
| `window.destroy()` | Closes the window at once, without asking. Closing the only window ends the app. |

Each request is carried out on the next frame, and the window manager has the last word. It
can ignore `minimize` or `maximize`; Hyprland ignored `maximize` in testing.

## Asking before the window closes

`window.onCloseRequested(handler)` calls `handler` whenever the window is asked to close:
by its close button, by the window manager, or by `window.close()`. The handler decides:

- Return `false`, and the window stays open.
- Return a promise, and the window stays open until it resolves. A promise that resolves to
  anything but `false` then closes the window, so a handler can show a confirmation and
  answer when the user has chosen.
- Return anything else, or nothing, and the window may close.

It returns a function that removes the handler. With several handlers, every one has to
agree. A handler that throws is logged and counts as agreeing, so a bug cannot trap the user.

```svelte
<script lang="ts">
  import { window } from "@neoworks/svelte-gpui";

  let dirty = $state(false);

  // Registered while the component lives; the returned function removes it.
  $effect(() => window.onCloseRequested(() => (dirty ? ask() : undefined)));

  let answer: ((close: boolean) => void) | undefined;
  let asking = $state(false);
  function ask(): Promise<boolean> {
    asking = true;
    return new Promise((resolve) => (answer = resolve));
  }
</script>

{#if asking}
  <!-- Two buttons calling answer?.(false) and answer?.(true), in an overlay. -->
{/if}
```

While a promise is pending, further requests to close are ignored. `examples/demo` does this
for the Files section's note: close the window with unsaved changes, and it asks first.

## More windows

`Window` from `@neoworks/svelte-gpui/ui` opens another window while it is mounted, and
closes it when it is removed. Its children are drawn there, not where it sits. It is the
same app, so state is shared between the windows as it is between any two components:

```svelte
{#if open}
  <Window title="Inspector" width={320} height={480} onclose={() => (open = false)}>
    <Text>{selected.name}</Text>
  </Window>
{/if}
```

`style` lays out and paints the window's content, and without a `background` the window
is transparent. `title`, `width`, `height` and `appId` are read when it opens; `appId` is
the app's own unless it is set. When the user closes it, `onclose` fires and it stays
closed until the app removes it and mounts it again.

`start(App, {}, { window: false })` opens no window of the app's own, for an app that
draws only in its `Window`s. `window` above then has nothing to report. Such an app keeps running
when its last window closes, as a shell must while a tray or a monitor comes and goes, and with
`--watch` a rebuild replaces its windows even when none is open.

`examples/windows` has two windows sharing a count.

## Layer surfaces

With `layer`, a `Window` is a layer surface (wlr-layer-shell), the kind bars, docks,
wallpapers and notifications are. The compositor places it against its `anchor` edges and
stretches it along an axis whose `width` or `height` is 0 or left out:

```svelte
<Window monitor="DP-2" layer="top" anchor={["top", "left", "right"]} height={32}
        exclusiveZone={32} namespace="bar" style={{ background: "#18181b" }}>
  <Clock />
</Window>
```

| Prop | Meaning |
|---|---|
| `layer` | `"background"`, `"bottom"`, `"top"` or `"overlay"`. |
| `anchor` | The edges it sits against: any of `"top"`, `"bottom"`, `"left"`, `"right"`. |
| `exclusiveZone` | Room it keeps along its anchored edge, which other windows stay out of. |
| `margin` | Space from its anchor edges: one number, or top, right, bottom, left. |
| `keyboard` | `"none"` (the default), `"exclusive"` or `"on-demand"`. |
| `namespace` | The name compositor rules match it by, such as Hyprland's `layerrule`. |
| `monitor` | The monitor it opens on, by name. |

`layer`, `anchor`, `margin`, `namespace` and `monitor` are read when it opens; `width`,
`height`, `exclusiveZone` and `keyboard` follow changes. A `Window` that names a monitor
that is not connected does not open at all, rather than on whichever monitor the compositor
would choose. `examples/layer-shell` has a bar and a card on one monitor.

## Input regions

A layer surface takes the pointer over its whole area. A shell often has a window the size of the
output with only a panel in it, and everything else has to reach the windows below. Mark the
parts that take the pointer with `input` on a `View`:

```svelte
<Window monitor="DP-2" layer="top" anchor={["top", "bottom", "left", "right"]}>
  <View input style={{ alignSelf: "end", margin: 20, width: 300, height: 200 }}>…</View>
</Window>
```

- A window with no `input` view takes the pointer everywhere, as before.
- With some, the window takes it only inside those views' boxes. Touch input follows the same region.
- A window whose `input` views have no area is click-through as a whole. A tray that is closed
  keeps a zero-size `<View input style={{ width: 0, height: 0 }} />`, so the surface stays
  click-through while the tray is shut and the window is not one that takes the pointer
  everywhere again.
- The region follows the views as they move, grow or come and go, and is read from where
  they were laid out, so a view with `display: "none"` or one that an ancestor hides adds
  nothing. A `View` inside a nested `Window` belongs to that one.

To dismiss a tray when the user clicks away, put a near-transparent `input` view that fills
the window under the panel: its press is the click-away.

`examples/shell` has a scene for this (`SCENE=regions`).

## Popups that rise above a window

A `Popover` or `Overlay` is drawn inside its window and cut off at the window's edge. A dock that
is 96 pixels tall has no room above itself for a menu. There is no separate popup surface: give
the window the room, and let the pointer through where it is empty.

```svelte
<Window layer="top" anchor={["bottom", "left", "right"]} height={360} exclusiveZone={96}>
  <View style={{ flex: 1, justify: "end", align: "center" }}>
    <View input>
      <Popover side="top" input>…</Popover>
    </View>
  </View>
</Window>
```

The window is 360 pixels tall, but `exclusiveZone` keeps other windows out of only the dock's 96, and
`input` on the dock and on the popover leaves the transparent rest click-through. `input` on a
`Popover` or `Overlay` marks the box it is open in, and nothing while it is closed. A window that
has to follow how big its content is can change its `height`, which follows changes.

The alternative is a second `Window` on the `overlay` layer for the popup, which opens, closes and
is placed by the `margin` and `anchor` of its own. Use it when the popup belongs on another monitor
or has a lifetime of its own.

A native `xdg_popup` (the fork has one, `WindowKind::AnchoredPopup`) was not wired in. Its size is
fixed when it opens, it has to be opened from the input event that asks for a grab, and the
compositor decides where it lands, while a taller window with an input region needs nothing from
the compositor and keeps every `Popover` behaviour: flipping, outside presses and focus.

`examples/shell` has the dock as a scene (`SCENE=dock`).

## Locking the session

A `Window` with `lock` is a lock surface (ext-session-lock) on its `monitor`. The session is locked
while at least one lock window is mounted: the compositor then shows nothing but the lock
surfaces and sends input only to them. Mount one per monitor, and unlock by unmounting all of them
in the same update, which is what an unlock after a good password does:

```svelte
<script lang="ts">
  import { monitors, sessionLock } from "@neoworks/svelte-gpui";
  let locked = $state(false);
</script>

{#if locked}
  {#each monitors.list as monitor (monitor.name)}
    <Window lock monitor={monitor.name}>
      <LockScreen onunlock={() => (locked = false)} />
    </Window>
  {/each}
{/if}
```

The compositor sizes a lock surface and gives it the keyboard; `layer`, `anchor`, `width` and the
rest are ignored, and a lock window without a `monitor` does not open. It is read when it opens.

`sessionLock.state` says where the lock is, and is reactive:

| State | Meaning |
|---|---|
| `"unlocked"` | No lock window is mounted. |
| `"locking"` | Lock windows are mounted and the compositor has not confirmed the lock yet. |
| `"locked"` | The compositor confirmed the lock. Only now is the session safe to leave. |
| `"finished"` | Lock windows are mounted and none is left: the compositor refused or ended the lock (another locker holds it, or it was restored), or the host refused to open them. It returns to `"unlocked"` when the app unmounts them. |

A monitor plugged in while locked needs a lock window of its own, which the `each` above gives
it. Unmounting only some of the windows keeps the session locked.

A broken locker traps the session, so the host opens lock windows only in a process started with
`SVELTE_GPUI_ALLOW_SESSION_LOCK=1`. Without it a lock window does not open, a message says so,
and the state goes to `"finished"`; `scripts/shot.sh` never sets it, so a test cannot lock.
The Hyprland option `misc:allow_session_lock_restore = true` lets a new locker replace one that
crashed. Draw the lock screen's content as an ordinary component (`examples/shell` has
`LockSurface` and the scene `SCENE=lock`) and look at it in a normal window first.

## Windows that draw nothing

A window with no children, or whose content has no size, still maps: the host draws it as a
fully transparent buffer, so the compositor does not wait on it or send it a new size again.

## Monitors

`monitors.list` from `@neoworks/svelte-gpui` is the connected monitors, each with its `name`
(the connector, such as `DP-2`) and its `x`, `y`, `width` and `height` in logical pixels. It
is reactive, so a bar per monitor follows them as they are plugged in and out:

```svelte
{#each monitors.list as monitor (monitor.name)}
  <Window monitor={monitor.name} layer="top" anchor={["top", "left", "right"]} height={32}>
    <Bar />
  </Window>
{/each}
```

It is empty until the host has reported them, shortly after the app starts.

## What is not there

GPUI 0.2.2 on Wayland does not support these, so the API leaves them out rather than
pretending:

- **Position** (`setPosition`, `center`). A Wayland client has no say over where its window
  goes; the compositor places it.
- **Resizing a window that is open** (`setSize`). GPUI's `resize` changes only its own idea of
  the window's size. The compositor does not hear of it, and the window draws nothing until
  the compositor sends a size of its own. Open the window at the size you want instead.
- **Changing the minimum size later** (`setMinSize`). Wayland takes it when the window is
  created, and GPUI has no call to change it.
- **Making the window not resizable.** GPUI's `is_resizable` option is ignored on Linux.

Native menus, notifications and a tray icon are separate features.
