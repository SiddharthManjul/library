import { describe, expect, it } from 'vitest'
import { contentSecurityPolicy, isAppUrl, isSafeExternalUrl } from '../../src/main/security'

describe('content security policy', () => {
  it('is strict in production: no inline script, no network, no eval', () => {
    const csp = contentSecurityPolicy(false)
    expect(csp).toContain("default-src 'none'")
    expect(csp).toContain("connect-src 'none'")
    expect(csp).not.toContain('unsafe-inline')
    expect(csp).not.toContain('unsafe-eval')
    expect(csp).not.toMatch(/https?:/)
  })

  it('only opens the dev server origin in development', () => {
    const csp = contentSecurityPolicy(true, 'http://localhost:5173')
    expect(csp).toContain('http://localhost:5173')
    expect(csp).not.toContain('unsafe-eval')
  })
})

describe('urls', () => {
  it('opens only https links externally', () => {
    expect(isSafeExternalUrl('https://example.com/a')).toBe(true)
    for (const u of ['http://example.com', 'file:///etc/passwd', 'javascript:alert(1)', 'data:text/html,x', 'ftp://x', 'not a url', '']) expect(isSafeExternalUrl(u), u).toBe(false)
  })

  it('keeps navigation inside the app', () => {
    const app = 'file:///opt/library/resources/app/out/renderer/index.html'
    expect(isAppUrl(app, app)).toBe(true)
    expect(isAppUrl('file:///etc/passwd', app)).toBe(false)
    expect(isAppUrl('https://example.com', app)).toBe(false)
    expect(isAppUrl('http://localhost:5173/', 'http://localhost:5173/')).toBe(true)
    expect(isAppUrl('http://localhost:5174/', 'http://localhost:5173/')).toBe(false)
  })
})
