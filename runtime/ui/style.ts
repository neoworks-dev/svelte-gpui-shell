// The style object a primitive takes. It crosses to the host as JSON, which parses
// each distinct string once; host/src/style.rs reads exactly these names.

/** Pixels, a share of the parent (`"50%"`, `"full"`), or `"auto"`. */
export type Size = number | `${number}%` | "full" | "auto";

/** `"#rgb"`, `"#rrggbb"`, `"#rrggbbaa"`, `"rgb(…)"`, `"rgba(…)"` or `"transparent"`. */
export type Color = string;

/** One value for every side, `[vertical, horizontal]`, or `[top, right, bottom, left]`. */
export type Sides<T> = T | [T, T] | [T, T, T, T];

export type Align = "start" | "center" | "end" | "stretch" | "baseline";

/** A colour at the start or end of a gradient, or `[colour, position]` with the position from 0 to 1. */
export type GradientStop = Color | [Color, number];

/**
 * A straight blend between two colours. `angle` is in degrees, 0 pointing up and
 * increasing clockwise as in CSS; it defaults to 180, top to bottom.
 */
export interface LinearGradient {
  angle?: number;
  stops: [GradientStop, GradientStop];
}

/** A plain colour, or a gradient: `{ linear: { angle: 135, stops: ["#7c3aed", "#2563eb"] } }`. */
export type Background = Color | { linear: LinearGradient };

export interface Shadow {
  x?: number;
  y?: number;
  blur?: number;
  spread?: number;
  color?: Color;
}

/** How a canvas stroke or line is drawn. */
export interface Pen {
  color?: Color;
  width?: number;
}

export interface Transition {
  duration?: number;
  delay?: number;
  easing?: "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out";
}

export interface Style {
  // Layout. Every primitive is a flex container; views stack their children.
  display?: "flex" | "none";
  direction?: "row" | "column";
  wrap?: boolean;
  /** Grow from nothing to fill what is left, as React Native's `flex: 1`. */
  flex?: number;
  grow?: number;
  shrink?: number;
  basis?: Size;
  align?: Align;
  alignSelf?: Align;
  justify?: "start" | "center" | "end" | "between" | "around";
  gap?: number;
  rowGap?: number;
  columnGap?: number;
  padding?: Sides<number>;
  paddingX?: number;
  paddingY?: number;
  paddingTop?: number;
  paddingRight?: number;
  paddingBottom?: number;
  paddingLeft?: number;
  margin?: Sides<Size>;
  marginX?: Size;
  marginY?: Size;
  marginTop?: Size;
  marginRight?: Size;
  marginBottom?: Size;
  marginLeft?: Size;

  // Size and position.
  width?: Size;
  height?: Size;
  /** Width and height at once. */
  size?: Size;
  minWidth?: Size;
  minHeight?: Size;
  maxWidth?: Size;
  maxHeight?: Size;
  aspectRatio?: number;
  position?: "relative" | "absolute";
  inset?: Sides<Size>;
  top?: Size;
  right?: Size;
  bottom?: Size;
  left?: Size;
  overflow?: "visible" | "hidden";
  /** Moves the element from where layout put it, in pixels: `[x, y]`. */
  translate?: [number, number];
  /**
   * Draws the element and everything inside it larger or smaller, about its centre: every length
   * in pixels inside is multiplied. A box with `width` and `height` in pixels keeps the room it
   * had in its parent; any other grows or shrinks with its contents. In `hover`, it needs a
   * `transition`, and then eases.
   */
  scale?: number;
  /** Degrees clockwise about the centre. Only an `Icon` drawn from `svg` or `src` turns. */
  rotate?: number;

  // Paint.
  background?: Background;
  opacity?: number;
  borderWidth?: Sides<number>;
  borderTopWidth?: number;
  borderRightWidth?: number;
  borderBottomWidth?: number;
  borderLeftWidth?: number;
  borderColor?: Color;
  /** All corners, or `[topLeft, topRight, bottomRight, bottomLeft]`. */
  radius?: number | [number, number, number, number];
  shadow?: Shadow | Shadow[] | null;
  cursor?: "default" | "pointer" | "text" | "grab" | "grabbing" | "crosshair" | "col-resize" | "row-resize" | "not-allowed";
  /** Eases changes of colour and opacity, including hover: a duration in milliseconds, or the details. */
  transition?: number | Transition;

  // Text. Children inherit these.
  color?: Color;
  fontSize?: number;
  fontWeight?: number | "normal" | "medium" | "semibold" | "bold";
  /** `"sans"`, `"mono"`, or the name of an installed font. */
  fontFamily?: string;
  /** In pixels. */
  lineHeight?: number;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  textAlign?: "left" | "center" | "right";
  nowrap?: boolean;
  /** One line, cut off with an ellipsis. */
  truncate?: boolean;
  maxLines?: number;
  uppercase?: boolean;
}

/** A style as the host reads it, or nothing. */
export function encode(style: Style | null | undefined): string | undefined {
  if (style === null || style === undefined) {
    return undefined;
  }
  return JSON.stringify(style);
}

/** Lays styles over each other, later ones winning, skipping the ones that are off. */
export function merge(...styles: (Style | null | undefined | false)[]): Style {
  const out: Style = {};
  for (const style of styles) {
    if (style) {
      Object.assign(out, style);
    }
  }
  return out;
}
