<!--
  An icon, tinted with `color` or else the text colour around it. Pass SVG markup
  (`import gear from "./gear.svg"` gives it, with the build's text loader), the
  path of a file, or the `name` of an icon in the desktop's icon theme:

    <Icon name="firefox" size={32} />
    <Icon name="audio-volume-high-symbolic" size={16} />
    <Icon name={app.icon} fallback="application-x-executable" size={48} />

  A theme icon keeps its own colours, except a symbolic one (a name ending in
  `-symbolic`), which is a mask and takes the colour like any other icon. A `name`
  that is an absolute path is that file, as a `.desktop` file's `Icon=` can be.
-->
<script lang="ts">
  import { encode, type Color, type Style } from "./style";

  interface Props {
    svg?: string;
    src?: string;
    /** An icon theme name such as `firefox` or `audio-volume-high-symbolic`, or an absolute file path. */
    name?: string;
    /** The name to look up when `name` is not in the theme. */
    fallback?: string;
    /** The icon theme to look in instead of the desktop's current one. */
    theme?: string;
    /** Width and height in pixels; the font size around it otherwise. */
    size?: number;
    color?: Color;
    style?: Style;
  }

  let props: Props = $props();

  const attributes = $derived({
    svg: props.svg,
    src: props.src,
    name: props.name,
    fallback: props.fallback,
    theme: props.theme,
    look: encode({ size: props.size, color: props.color, ...props.style }),
  });
</script>

<g-icon {...attributes}></g-icon>
