// Piecewise-linear keyframes, for animations the style system has no property
// for: a list of {at, values} stops with `at` from 0 to 1, read at a progress.

export interface Keyframe {
  at: number
  values: Record<string, number>
}

export function sampleKeyframes(frames: Keyframe[], progress: number): Record<string, number> {
  const clamped = Math.min(1, Math.max(0, progress))
  for (let index = 1; index < frames.length; index += 1) {
    if (clamped <= frames[index].at) {
      return between(frames[index - 1], frames[index], clamped)
    }
  }
  return frames[frames.length - 1].values
}

function between(from: Keyframe, to: Keyframe, progress: number): Record<string, number> {
  const span = to.at - from.at
  let fraction = 1
  if (span > 0) {
    fraction = (progress - from.at) / span
  }
  const result: Record<string, number> = {}
  for (const key of Object.keys(from.values)) {
    result[key] = from.values[key] + (to.values[key] - from.values[key]) * fraction
  }
  return result
}
