import { colors } from '../../theme'

// The colour of each band `bandOf` names.
const BAND_COLORS: Record<string, string> = {
  good: colors.success,
  fair: colors.warning,
  poor: colors.error,
  severe: colors.accent,
}

export function bandColor(band: string): string {
  return BAND_COLORS[band]
}
