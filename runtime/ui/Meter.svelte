<!--
  A level bar: a track filled up to `value` between `min` and `max`. The fill eases to a new value,
  and `levels` change its colour as the value passes them.

    <Meter value={volume} />
    <Meter value={battery} levels={[{ upTo: 15, color: "#f87171" }, { upTo: 40, color: "#fbbf24" }, { upTo: 100, color: "#4ade80" }]} />

  A column of `Meter`s is `vertical`, filling from the bottom; give it a `height`.
-->
<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import { Tween } from "svelte/motion";
  import { colors } from "./controls-look";
  import { levelColor, share, type Level } from "./controls";
  import type { Style } from "./style";
  import View from "./View.svelte";

  interface Props {
    value: number;
    min?: number;
    max?: number;
    /** Colours by value: the first level the value does not exceed. */
    levels?: Level[];
    /** The fill's colour when no level applies. */
    color?: string;
    vertical?: boolean;
    /** Milliseconds the fill takes to move; 0 moves it at once. The default is 200. */
    duration?: number;
    /** The track. It is 6 pixels thick. */
    style?: Style;
    fillStyle?: Style;
  }

  let { value, min = 0, max = 100, levels, color = colors.accent, vertical = false, duration = 200, style, fillStyle }: Props = $props();

  const shown = Tween.of(() => share(value, { min, max, step: 0 }) * 100, { duration: () => duration, easing: cubicOut });
  const fill = $derived(levelColor(value, levels, color));
</script>

<View
  style={{
    ...(vertical ? { width: 6, justify: "end" } : { height: 6 }),
    radius: 3,
    background: "#ffffff1a",
    overflow: "hidden",
    ...style,
  }}
>
  <View
    style={{
      ...(vertical ? { height: `${shown.current}%` } : { width: `${shown.current}%`, height: "full" }),
      radius: 3,
      background: fill,
      transition: 200,
      ...fillStyle,
    }}
  />
</View>
