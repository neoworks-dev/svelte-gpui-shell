import type { Style } from '@neoworks/svelte-gpui/ui'
import { alpha, colors } from './theme'

// The look the desktop cards (weather, air quality) share.

export const cardStyle: Style = {
  flex: 1,
  gap: 10,
  padding: 14,
  radius: 24,
  background: alpha(colors.surface, 0.65),
  borderWidth: 1,
  borderColor: alpha(colors.content, 0.1),
  shadow: { y: 10, blur: 20, color: alpha('#000000', 0.35) },
  color: colors.content,
  fontSize: 13,
}

export const titleStyle: Style = { fontSize: 13, fontWeight: 500 }

export const hintStyle: Style = { fontSize: 11, color: alpha(colors.content, 0.6) }

export const placeStyle: Style = { fontSize: 13, fontWeight: 500, truncate: true }

export const readingStyle: Style = { fontSize: 44, fontWeight: 300, tabularNumbers: true }

export const conditionStyle: Style = { fontSize: 13, color: alpha(colors.content, 0.9), truncate: true }

export const rangeStyle: Style = { fontSize: 11, color: alpha(colors.content, 0.6) }

export const stripLabelStyle: Style = { fontSize: 11, color: alpha(colors.content, 0.6) }

export const stripValueStyle: Style = { fontSize: 13, tabularNumbers: true }

export const dayLabelStyle: Style = { fontSize: 13, color: alpha(colors.content, 0.7) }

export const dayHighStyle: Style = { fontSize: 13, tabularNumbers: true }
