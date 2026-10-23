<!--
  A video file or stream, played by the host through GStreamer and drawn in its box as `fit` says.
  It loops and has no sound unless told otherwise, which is what a live wallpaper wants. The host
  decodes with VA-API when the system has it, and scales every frame to the size the box shows,
  so a video larger than the screen costs little more than one that fits.

    <Video src="/home/me/loop.mp4" fit="cover" rate={1.5} paused={onBattery}
           style={{ position: "absolute", inset: 0 }} />

  Without a size of its own the box is as large as `style` makes it, and empty until the
  video's first frame is there. A video that cannot be opened calls `onerror` and draws nothing.
-->
<script lang="ts">
  import { encode, type Style } from "./style";
  import { listener } from "./events";

  interface Props {
    style?: Style;
    /** A path (relative ones start in the app's own directory), a `file://` URI, or a URL GStreamer can read. */
    src?: string;
    /** Starts again when it ends. The default is on. */
    loop?: boolean;
    /** Plays without sound. The default is on; with it off, the audio plays through the system's output. */
    muted?: boolean;
    paused?: boolean;
    /** How fast it plays, from 0.1 to 8. The default is 1. A change restarts from the current position. */
    rate?: number;
    /** How the picture fills the box: all of it showing (`"contain"`, the default), the box full (`"cover"`), or stretched (`"fill"`). */
    fit?: "contain" | "cover" | "fill";
    /** The most frames a second it decodes and draws, for a video that is faster than the screen needs. */
    maxFps?: number;
    /** The video opened: `duration` is in milliseconds. */
    onready?: (event: { duration: number }) => void;
    /** It played to its end, which only happens without `loop`. */
    onend?: () => void;
    /** It could not be opened or stopped playing. */
    onerror?: (event: { message: string }) => void;
  }

  let props: Props = $props();

  const attributes = $derived({
    look: encode(props.style),
    src: props.src,
    loop: props.loop === false ? "false" : undefined,
    muted: props.muted === false ? "false" : undefined,
    paused: props.paused ? "" : undefined,
    rate: props.rate === undefined ? undefined : String(props.rate),
    fit: props.fit,
    "max-fps": props.maxFps === undefined ? undefined : String(props.maxFps),
    onready: listener(props.onready, (event: { duration: number }) => event),
    onend: listener(props.onend, () => undefined),
    onerror: listener(props.onerror, (event: { message: string }) => event),
  });
</script>

<g-video {...attributes}></g-video>
