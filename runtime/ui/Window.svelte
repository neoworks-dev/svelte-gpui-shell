<!--
  A window of its own. Its children are drawn there instead of where it sits, and it opens
  while it is mounted and closes when it is removed. It is part of the same app as the
  window around it, so state is shared the way it is between any two components.

  `style` lays out and paints the window's content; without a background the window is
  transparent. `title`, `width`, `height` and `appId` are read when it opens.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { encode, type Style } from "./style";
  import { listener } from "./events";

  interface Props {
    style?: Style;
    title?: string;
    /** The content's size in logical pixels when it opens. The default is 400 by 300. */
    width?: number;
    height?: number;
    /** The id the desktop matches window rules against. */
    appId?: string;
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
    onclose: listener(props.onclose, () => undefined),
  });
</script>

<g-window {...attributes}>{@render props.children?.()}</g-window>
