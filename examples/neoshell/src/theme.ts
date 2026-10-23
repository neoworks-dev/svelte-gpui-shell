import type { Style } from '@neoworks/svelte-gpui/ui'

// The neoworks palette, shared by every view. The values are the ones the
// GTK stylesheet declared as custom properties.

export const colors = {
  canvas: '#000000',
  elevated: '#0b0b0d',
  surface: '#141416',
  content: '#fafafa',
  secondary: '#1c1c1f',
  secondaryContent: '#a1a1aa',
  neutralContent: '#71717a',
  accent: '#a78bfa',
  info: '#60a5fa',
  success: '#4ade80',
  warning: '#fbbf24',
  error: '#f87171',
}

export const radii = {
  field: 6,
  box: 8,
}

// A colour at a given opacity, written the way the host reads it: #rrggbbaa.
export function alpha(hex: string, opacity: number): string {
  const channel = Math.round(Math.min(1, Math.max(0, opacity)) * 255)
  return `${hex}${channel.toString(16).padStart(2, '0')}`
}

// A layer surface has to paint something, or the compositor never maps it. One
// hundredth of alpha is a real paint that is still invisible; the blur rules
// ignore anything under 0.1.
export const windowBackground = alpha('#000000', 0.01)

export const surfaceText: Style = {
  color: colors.content,
  fontSize: 13,
}
