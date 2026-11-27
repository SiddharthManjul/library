// @vitest-environment jsdom
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { App } from '../../src/renderer/App'
import type { LibraryBridge } from '../../src/shared/bridge'

function bridge(over: Partial<LibraryBridge> & { session?: unknown; fail?: boolean } = {}): LibraryBridge {
  const { session = { state: 'signed_out' }, fail = false, ...rest } = over
  return {
    invoke: vi.fn(async (m: string) => {
      if (fail) throw new Error('down')
      if (m === 'node.info') return { name: 'library-node', version: '0.0.1', protocol: 1, platform: 'linux-x86_64' }
      if (m === 'session.state') return session
      throw new Error('unexpected ' + m)
    }),
    onNodeStatus: () => () => undefined,
    platform: 'linux',
    previewShell: false,
    windowControl: vi.fn(async () => undefined),
    ...rest,
  }
}
const mount = (b: LibraryBridge) => {
  window.library = b
  return render(<App />)
}

describe('sign-in screen', () => {
  it('shows when no member is signed in, with the node status in words', async () => {
    mount(bridge())
    expect(await screen.findByRole('heading', { name: 'Sign in to Library' })).toBeTruthy()
    expect(await screen.findByText(/Local node ready · v0\.0\.1/)).toBeTruthy()
    for (const p of ['Google', 'Microsoft', 'GitHub']) expect((screen.getByRole('button', { name: `Continue with ${p}` }) as HTMLButtonElement).disabled).toBe(true)
  })

  it('says plainly when the node did not start', async () => {
    mount(bridge({ fail: true }))
    expect((await screen.findByRole('alert')).textContent).toMatch(/local node did not start/i)
    expect(screen.getByText('Local node unavailable')).toBeTruthy()
  })

  it('does not show the shell controls before sign-in', async () => {
    mount(bridge())
    await screen.findByRole('heading', { name: 'Sign in to Library' })
    expect(screen.queryByRole('navigation', { name: 'Activity bar' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Toggle sidebar' })).toBeNull()
  })
})

describe('shell', () => {
  const signedIn = { state: 'signed_in', tenant_id: 't', user_id: 'u' }

  it('replaces sign-in once the node reports a member', async () => {
    mount(bridge({ session: signedIn }))
    expect(await screen.findByRole('navigation', { name: 'Activity bar' })).toBeTruthy()
    expect(screen.queryByRole('heading', { name: 'Sign in to Library' })).toBeNull()
  })

  it('every control has an accessible name', async () => {
    mount(bridge({ previewShell: true }))
    await screen.findByRole('navigation', { name: 'Activity bar' })
    for (const b of screen.getAllByRole('button')) expect((b.getAttribute('aria-label') ?? b.textContent ?? '').trim(), b.outerHTML).not.toBe('')
    for (const s of screen.getAllByRole('separator')) expect(s.getAttribute('aria-label')).toBeTruthy()
  })

  it('toggles panes from the keyboard with Ctrl and shows Ctrl, not the Mac key, on Windows and Linux', async () => {
    mount(bridge({ previewShell: true }))
    await screen.findByRole('navigation', { name: 'Activity bar' })
    expect(screen.getByText('Ctrl+K')).toBeTruthy()
    expect(screen.getByRole('complementary', { name: /sidebar/ })).toBeTruthy()
    fireEvent.keyDown(window, { code: 'KeyB', ctrlKey: true })
    expect(screen.queryByRole('complementary', { name: /sidebar/ })).toBeNull()
    fireEvent.keyDown(window, { code: 'KeyJ', ctrlKey: true })
    expect(screen.getByRole('region', { name: 'Panel' })).toBeTruthy()
    fireEvent.keyDown(window, { code: 'KeyB', ctrlKey: true, altKey: true })
    expect(screen.queryByRole('complementary', { name: 'Agent pane' })).toBeNull()
  })

  it('switches the sidebar when an activity bar view is chosen', async () => {
    mount(bridge({ previewShell: true }))
    const nav = await screen.findByRole('navigation', { name: 'Activity bar' })
    fireEvent.click(within(nav).getByRole('button', { name: 'Brain' }))
    expect(within(screen.getByRole('complementary', { name: 'Brain sidebar' })).getByRole('heading', { name: 'BRAIN' })).toBeTruthy()
    expect(within(nav).getByRole('button', { name: 'Brain' }).getAttribute('aria-current')).toBe('page')
  })

  it('resizes with the keyboard within the limits', async () => {
    mount(bridge({ previewShell: true }))
    const sep = await screen.findByRole('separator', { name: 'Resize sidebar' })
    expect(sep.getAttribute('aria-valuenow')).toBe('264')
    fireEvent.keyDown(sep, { key: 'ArrowRight' })
    expect(screen.getByRole('separator', { name: 'Resize sidebar' }).getAttribute('aria-valuenow')).toBe('280')
    fireEvent.keyDown(sep, { key: 'End' })
    expect(screen.getByRole('separator', { name: 'Resize sidebar' }).getAttribute('aria-valuenow')).toBe('420')
  })
})

describe('window controls', () => {
  it('are drawn on Linux and call the bridge', async () => {
    const b = bridge({ previewShell: true })
    mount(b)
    await screen.findByRole('navigation', { name: 'Activity bar' })
    fireEvent.click(screen.getByRole('button', { name: 'Minimise' }))
    fireEvent.click(screen.getByRole('button', { name: 'Maximise' }))
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(vi.mocked(b.windowControl).mock.calls.map((c) => c[0])).toEqual(['minimize', 'maximize', 'close'])
  })

  it('are left to Windows, which draws its own', async () => {
    mount(bridge({ previewShell: true, platform: 'win32' }))
    await screen.findByRole('navigation', { name: 'Activity bar' })
    expect(screen.queryByRole('button', { name: 'Close' })).toBeNull()
    expect(screen.getByText('Ctrl+K')).toBeTruthy()
  })

  it('show on the sign-in screen too', async () => {
    mount(bridge())
    await screen.findByRole('heading', { name: 'Sign in to Library' })
    expect(screen.getByRole('button', { name: 'Close' })).toBeTruthy()
  })
})
