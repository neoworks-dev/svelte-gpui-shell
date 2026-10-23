<!--
  An on and off switch whose knob slides. It is pressed by the pointer, or with Enter or Space
  while it has focus. Children make a label that toggles it too.

    <Switch bind:checked={notify} />
    <Switch bind:checked={notify}>Notifications</Switch>
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { Tween } from "svelte/motion";
  import Button from "./Button.svelte";
  import View from "./View.svelte";
  import { colors } from "./controls-look";
  import type { Style } from "./style";

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    /** The track's height in pixels, which sets the rest. The default is 20. */
    size?: number;
    onchange?: (checked: boolean) => void;
    /** The whole control: the track and the label. */
    style?: Style;
    hover?: Style;
    focus?: Style;
    /** The track. */
    trackStyle?: Style;
    /** Laid over `trackStyle` while the switch is on. */
    checkedStyle?: Style;
    knobStyle?: Style;
    children?: Snippet;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    size = 20,
    onchange,
    style,
    hover,
    focus,
    trackStyle,
    checkedStyle,
    knobStyle,
    children,
  }: Props = $props();

  const width = $derived(Math.round(size * 1.8));
  const knob = $derived(size - 4);
  const slide = Tween.of(() => (checked ? width - size : 0), { duration: 160, easing: cubicOut });

  function toggle(): void {
    checked = !checked;
    onchange?.(checked);
  }
</script>

<Button
  {disabled}
  disabledStyle={{ opacity: 0.4 }}
  style={{ gap: 10, radius: 6, cursor: "pointer", ...style }}
  {hover}
  focus={{ background: "#ffffff10", ...focus }}
  onpress={toggle}
>
  <View
    style={{
      width,
      height: size,
      shrink: 0,
      radius: size / 2,
      padding: 2,
      background: colors.track,
      transition: 160,
      ...trackStyle,
      ...(checked ? { background: colors.accent, ...checkedStyle } : undefined),
    }}
  >
    <View style={{ size: knob, radius: knob / 2, background: colors.knob, translate: [slide.current, 0], ...knobStyle }} />
  </View>
  {@render children?.()}
</Button>
