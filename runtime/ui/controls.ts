// The arithmetic under `Slider` and `Meter`, apart from the components so it can be tested.

export interface Range {
  min: number;
  max: number;
  /** The distance between values; 0 or less allows any value. */
  step: number;
}

/** A colour for a meter whose value is at most `upTo`. */
export interface Level {
  upTo: number;
  color: string;
}

const clamp = (value: number, low: number, high: number): number => Math.min(Math.max(value, low), high);

/** Decimal places in a number as written, so that 0.1 steps add up to 0.3 and not 0.30000000000000004. */
function decimals(value: number): number {
  const [, fraction = ""] = String(value).split(".");
  return fraction.length;
}

/** The nearest value to `value` that is a whole number of steps above `min`, kept inside the range. */
export function snap(value: number, { min, max, step }: Range): number {
  if (!(max > min)) {
    return min;
  }
  const inside = clamp(value, min, max);
  if (!(step > 0)) {
    return inside;
  }
  const snapped = min + Math.round((inside - min) / step) * step;
  const places = Math.max(decimals(step), decimals(min));
  return clamp(Number(snapped.toFixed(places)), min, max);
}

/** Where `value` lies between `min` and `max`, from 0 to 1. */
export function share(value: number, { min, max }: Range): number {
  if (!(max > min)) {
    return 0;
  }
  return clamp((value - min) / (max - min), 0, 1);
}

/**
 * The value under a pointer `x` pixels along a slider `width` wide. The knob's centre travels
 * between half a knob from each end, so the pointer is mapped over that. A track no wider than its
 * knob, such as one not yet measured, gives `min`.
 */
export function valueAt(x: number, width: number, knob: number, range: Range): number {
  if (width <= knob) {
    return snap(range.min, range);
  }
  const travel = width - knob;
  const along = clamp((x - knob / 2) / travel, 0, 1);
  return snap(range.min + along * (range.max - range.min), range);
}

/** The value after a key, or `undefined` when the key does not move a slider. */
export function nudge(value: number, key: string, range: Range): number | undefined {
  const unit = range.step > 0 ? range.step : (range.max - range.min) / 100;
  switch (key) {
    case "ArrowLeft":
    case "ArrowDown":
      return snap(value - unit, range);
    case "ArrowRight":
    case "ArrowUp":
      return snap(value + unit, range);
    case "PageDown":
      return snap(value - unit * 10, range);
    case "PageUp":
      return snap(value + unit * 10, range);
    case "Home":
      return range.min;
    case "End":
      return snap(range.max, range);
    default:
      return undefined;
  }
}

/** The colour of the first level the value does not exceed, the last one when it exceeds them all, else `fallback`. */
export function levelColor(value: number, levels: Level[] | undefined, fallback: string): string {
  if (levels === undefined || levels.length === 0) {
    return fallback;
  }
  const ordered = [...levels].sort((a, b) => a.upTo - b.upTo);
  const reached = ordered.find((level) => value <= level.upTo);
  return (reached ?? ordered[ordered.length - 1]).color;
}
