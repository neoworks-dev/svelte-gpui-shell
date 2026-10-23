<!--
  A slider: drag the knob or press the track, or use the arrow keys, Page Up and Down, Home and End
  while it has focus. The pointer stays captured while dragging, so it can leave the track.
  It is as wide as its parent in a column; give it a `width` or `flex: 1` in a row.

    <Slider bind:value={volume} />
    <Slider bind:value={opacity} min={0} max={1} step={0.05} style={{ width: 200 }} />
-->
<script lang="ts">
  import { colors } from "./controls-look";
  import { nudge, share, valueAt, type Range } from "./controls";
  import type { KeyEvent, Layout, PointerEvent } from "./events";
  import type { Style } from "./style";
  import View from "./View.svelte";

  interface Props {
    value?: number;
    min?: number;
    max?: number;
    /** The distance between values, which a drag snaps to. The default is 1; 0 allows any value. */
    step?: number;
    disabled?: boolean;
    /** The knob's width and height in pixels. The default is 16. */
    knobSize?: number;
    /** Called as the value changes, while dragging too. */
    onchange?: (value: number) => void;
    /** Called when a drag is released. */
    oncommit?: (value: number) => void;
    /** The control, which is `knobSize` plus 4 high and takes the pointer. */
    style?: Style;
    trackStyle?: Style;
    /** The part of the track before the knob. */
    fillStyle?: Style;
    knobStyle?: Style;
  }

  let {
    value = $bindable(0),
    min = 0,
    max = 100,
    step = 1,
    disabled = false,
    knobSize = 16,
    onchange,
    oncommit,
    style,
    trackStyle,
    fillStyle,
    knobStyle,
  }: Props = $props();

  let width = $state(0);
  let dragging = $state(false);
  let focused = $state(false);

  const range: Range = $derived({ min, max, step });
  const along = $derived(share(value, range));
  const travel = $derived(Math.max(width - knobSize, 0));

  function change(next: number): void {
    if (next === value) {
      return;
    }
    value = next;
    onchange?.(next);
  }

  function press(event: PointerEvent): void {
    if (disabled) {
      return;
    }
    dragging = true;
    change(valueAt(event.x, width, knobSize, range));
  }

  function move(event: PointerEvent): void {
    if (dragging) {
      change(valueAt(event.x, width, knobSize, range));
    }
  }

  function release(): void {
    if (!dragging) {
      return;
    }
    dragging = false;
    oncommit?.(value);
  }

  function key(event: KeyEvent): void {
    const next = disabled ? undefined : nudge(value, event.key, range);
    if (next === undefined) {
      return;
    }
    event.preventDefault();
    change(next);
    oncommit?.(next);
  }
</script>

<View
  focusable={!disabled}
  buttons={["left"]}
  style={{ height: knobSize + 4, justify: "center", cursor: "pointer", ...(disabled ? { opacity: 0.4 } : undefined), ...style }}
  ondown={press}
  onmove={move}
  onup={release}
  onkey={key}
  onresize={(layout: Layout) => (width = layout.width)}
  onfocus={() => (focused = true)}
  onblur={() => (focused = false)}
>
  <View style={{ height: 4, radius: 2, background: colors.track, ...trackStyle }}>
    <View style={{ width: knobSize / 2 + travel * along, height: 4, radius: 2, background: colors.accent, ...fillStyle }} />
  </View>
  <View
    style={{
      position: "absolute",
      left: travel * along,
      top: 2,
      size: knobSize,
      radius: knobSize / 2,
      background: colors.knob,
      borderWidth: dragging || focused ? 3 : 0,
      borderColor: colors.accent,
      shadow: { y: 1, blur: 3, color: "#00000080" },
      ...knobStyle,
    }}
  />
</View>
