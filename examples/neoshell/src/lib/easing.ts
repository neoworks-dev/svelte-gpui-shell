// CSS's cubic-bezier(x1, y1, x2, y2) as an easing function for Tween: the time
// is the curve's x, solved by Newton's method with a bisection fallback, and
// the result its y. Control points above 1 overshoot, which is what the
// island's spring-like opening uses.

const NEWTON_STEPS = 6
const BISECTION_STEPS = 24
const EPSILON = 1e-6

export function cubicBezier(x1: number, y1: number, x2: number, y2: number): (time: number) => number {
  const sampleX = (parameter: number) => axis(parameter, x1, x2)
  const sampleY = (parameter: number) => axis(parameter, y1, y2)
  const slopeX = (parameter: number) => slope(parameter, x1, x2)
  return (time) => {
    if (time <= 0) {
      return 0
    }
    if (time >= 1) {
      return 1
    }
    return sampleY(solve(time, sampleX, slopeX))
  }
}

// One coordinate of the curve at a parameter: a cubic Bezier from 0 to 1.
function axis(parameter: number, first: number, second: number): number {
  const inverse = 1 - parameter
  return 3 * inverse * inverse * parameter * first + 3 * inverse * parameter * parameter * second + parameter ** 3
}

function slope(parameter: number, first: number, second: number): number {
  const inverse = 1 - parameter
  return 3 * inverse * inverse * first + 6 * inverse * parameter * (second - first) + 3 * parameter * parameter * (1 - second)
}

function solve(
  target: number,
  sampleX: (parameter: number) => number,
  slopeX: (parameter: number) => number,
): number {
  let parameter = target
  for (let step = 0; step < NEWTON_STEPS; step += 1) {
    const error = sampleX(parameter) - target
    if (Math.abs(error) < EPSILON) {
      return parameter
    }
    const derivative = slopeX(parameter)
    if (Math.abs(derivative) < EPSILON) {
      break
    }
    parameter -= error / derivative
  }
  return bisect(target, sampleX)
}

function bisect(target: number, sampleX: (parameter: number) => number): number {
  let low = 0
  let high = 1
  let parameter = target
  for (let step = 0; step < BISECTION_STEPS; step += 1) {
    const value = sampleX(parameter)
    if (Math.abs(value - target) < EPSILON) {
      return parameter
    }
    if (value < target) {
      low = parameter
    } else {
      high = parameter
    }
    parameter = (low + high) / 2
  }
  return parameter
}
