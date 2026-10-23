<!--
  A window of its own. Its children are drawn there instead of where it sits, and it opens
  while it is mounted and closes when it is removed. It is part of the same app as the
  window around it, so state is shared the way it is between any two components.

  `style` lays out and paints the window's content; without a background the window is
  transparent. `title`, `appId`, `monitor`, `layer`, `anchor`, `margin` and `namespace` are
  read when it opens; `width`, `height`, `exclusiveZone` and `keyboard` follow changes.

  With `layer` it is a layer surface (wlr-layer-shell), as bars, docks and wallpapers are:
  the compositor places it against the `anchor` edges, and stretches it along an axis whose
  size is 0 or left out.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { encode, type Style } from "./style";
  import { listener } from "./events";

  interface Props {
    style?: Style;
    title?: string;
    /**
     * The content's size in logical pixels. The default is 400 by 300, and for a layer
     * surface 0, which the compositor fills in along anchored axes.
     */
    width?: number;
    height?: number;
    /** The id the desktop matches window rules against. The app's own by default. */
    appId?: string;
    /** The monitor it opens on, by name (`monitors.list`). The compositor picks without one. */
    monitor?: string;
    /** Makes it a layer surface on this layer. */
    layer?: "background" | "bottom" | "top" | "overlay";
    /** The edges a layer surface sits against. */
    anchor?: ("top" | "bottom" | "left" | "right")[];
    /** Room a layer surface keeps for itself along its anchored edge, which windows stay out of. */
    exclusiveZone?: number;
    /** Space between a layer surface and its anchor edges: one number, or top, right, bottom, left. */
    margin?: number | [number, number, number, number];
    /** Whether a layer surface takes the keyboard. The default is `"none"`. */
    keyboard?: "none" | "exclusive" | "on-demand";
    /** The name compositor rules match a layer surface by (Hyprland's `layerrule`). */
    namespace?: string;
    /** The user or the compositor closed it. It stays closed until it is removed and mounted again. */
    onclose?: () => void;
    children?: Snippet;
  }

  let props: Props = $props();

  const attributes = $derived({
    look: encode(props.style),
    title: props.title,
    width: props.width,
    height: props.height,
    "app-id": props.appId,
    monitor: props.monitor,
    layer: props.layer,
    anchor: props.anchor?.join(" "),
    "exclusive-zone": props.exclusiveZone,
    margin: typeof props.margin === "number" ? props.margin : props.margin?.join(" "),
    keyboard: props.keyboard,
    namespace: props.namespace,
    onclose: listener(props.onclose, () => undefined),
  });
</script>

<g-window {...attributes}>{@render props.children?.()}</g-window>
