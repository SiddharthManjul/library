export const THEMES = ['dark', 'light', 'dim'] as const
export type Theme = (typeof THEMES)[number]

/** Ice Dark is the default. `?theme=` and the saved choice can switch it; anything else falls back. */
export function pickTheme(search: string, saved: string | null): Theme {
  const asked = new URLSearchParams(search).get('theme')
  for (const c of [asked, saved]) if (c && (THEMES as readonly string[]).includes(c)) return c as Theme
  return 'dark'
}
