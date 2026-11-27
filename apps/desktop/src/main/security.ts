/** Content security policy for the renderer. Production is as strict as it can be: no remote content. */
export function contentSecurityPolicy(isDev: boolean, devOrigin = 'http://localhost:5173'): string {
  const dev = isDev
  return [
    "default-src 'none'",
    `script-src 'self'${dev ? " 'unsafe-inline'" : ''}`,
    "style-src 'self'" + (dev ? " 'unsafe-inline'" : ''),
    "font-src 'self'",
    "img-src 'self' data:",
    `connect-src ${dev ? `'self' ${devOrigin} ${devOrigin.replace('http', 'ws')}` : "'none'"}`,
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join('; ')
}

/** Only https links leave the app, and they open in the system browser. */
export function isSafeExternalUrl(raw: string): boolean {
  try {
    return new URL(raw).protocol === 'https:'
  } catch {
    return false
  }
}

/** The renderer may navigate only within the app's own origin. */
export function isAppUrl(target: string, appUrl: string): boolean {
  try {
    const a = new URL(target)
    const b = new URL(appUrl)
    return a.protocol === b.protocol && a.host === b.host && (a.protocol !== 'file:' || a.pathname === b.pathname)
  } catch {
    return false
  }
}
