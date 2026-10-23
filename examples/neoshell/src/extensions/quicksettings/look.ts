import type { Style } from '@neoworks/svelte-gpui/ui'
import { alpha, colors } from '../../theme'

// The styles the tray's pages share. Each was a rule in the GTK stylesheet.

export const PANEL_WIDTH = 328

export const panelStyle: Style = {
  background: alpha(colors.elevated, 0.9),
  borderWidth: 1,
  borderColor: alpha(colors.content, 0.12),
  radius: 24,
  padding: 10,
  gap: 10,
  color: colors.content,
  fontSize: 13,
  shadow: { y: 25, blur: 50, color: alpha('#000000', 0.6) },
}

export const pageHeaderStyle: Style = {
  direction: 'row',
  align: 'center',
  gap: 8,
  paddingX: 2,
}

export const roundButtonStyle: Style = {
  width: 32,
  height: 32,
  radius: 16,
  align: 'center',
  justify: 'center',
  transition: 120,
}

export const roundButtonHover: Style = { background: alpha(colors.content, 0.1) }

export const pageTitleStyle: Style = { flex: 1, minWidth: 0, fontSize: 14, fontWeight: 500, truncate: true }

export const failureStyle: Style = {
  fontSize: 11,
  paddingX: 12,
  paddingY: 8,
  radius: 8,
  color: colors.error,
  background: alpha(colors.error, 0.15),
}

export const emptyStyle: Style = {
  fontSize: 12,
  paddingX: 4,
  paddingY: 24,
  color: alpha(colors.content, 0.5),
  textAlign: 'center',
}

export const rowStyle: Style = {
  direction: 'row',
  align: 'center',
  gap: 12,
  paddingX: 10,
  paddingY: 8,
  radius: 12,
  transition: 120,
}

export const rowHover: Style = { background: alpha(colors.content, 0.08) }

export const activeRow: Style = { background: alpha(colors.info, 0.2) }

export const rowTitleStyle: Style = { fontSize: 12, truncate: true }

export const rowNoteStyle: Style = { fontSize: 10, color: alpha(colors.content, 0.5) }

export const sectionStyle: Style = {
  fontSize: 10,
  paddingX: 10,
  paddingY: 4,
  color: alpha(colors.content, 0.4),
}

export const detailsStyle: Style = {
  background: alpha(colors.surface, 0.6),
  radius: 12,
  paddingX: 12,
  paddingY: 10,
  gap: 6,
  fontSize: 11,
}

export const buttonStyle: Style = {
  fontSize: 12,
  paddingX: 12,
  paddingY: 4,
  radius: 6,
  transition: 120,
}

export const buttonHover: Style = { background: alpha(colors.content, 0.1) }

export const primaryButtonStyle: Style = { background: colors.info, color: colors.canvas }

export const primaryButtonHover: Style = { background: alpha(colors.info, 0.85) }

export const separatorStyle: Style = { height: 1, background: alpha(colors.content, 0.1) }

export const listHeight = 256
