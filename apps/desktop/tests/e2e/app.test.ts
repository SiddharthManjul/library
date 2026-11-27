// End to end: the real Electron app, the real node binary. Needs a display (WSLg, a desktop, or xvfb).
// `--no-sandbox` is for this test only (CI containers lack the SUID helper); installed apps run sandboxed.
import { existsSync, mkdirSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { _electron as electron, type ElectronApplication, type Page } from 'playwright-core'
import { afterEach, describe, expect, it } from 'vitest'

const root = resolve(__dirname, '../..')
const exe = process.platform === 'win32' ? 'library-node.exe' : 'library-node'
const nodeBin = resolve(root, '../../target/debug', exe)
const shots = join(root, 'tests/e2e/shots')

let app: ElectronApplication | null = null
afterEach(async () => {
  await app?.close().catch(() => undefined)
  app = null
})

async function launch(extraEnv: Record<string, string> = {}): Promise<Page> {
  if (!existsSync(nodeBin)) throw new Error(`build the node first: cargo build -p library-node (${nodeBin})`)
  mkdirSync(shots, { recursive: true })
  app = await electron.launch({
    args: [join(root, 'out/main/index.js'), '--no-sandbox'],
    cwd: root,
    env: { ...process.env, LIBRARY_NODE_PATH: nodeBin, ...extraEnv },
  })
  const page = await app.firstWindow()
  // The window stays hidden until its first paint; interacting before that makes screenshots race.
  await expect.poll(() => app!.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.isVisible() ?? false), { timeout: 15_000 }).toBe(true)
  await page.setViewportSize({ width: 1440, height: 900 })
  return page
}

describe('library desktop', () => {
  it('starts the node and shows the sign-in screen', async () => {
    const page = await launch()
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    await page.getByText(/Local node ready/).waitFor()
    await page.screenshot({ path: join(shots, 'signin-dark.png') })
    // Sign-in providers are not connected yet, so they cannot be pressed.
    for (const name of ['Google', 'Microsoft', 'GitHub']) {
      expect(await page.getByRole('button', { name: `Continue with ${name}` }).isDisabled()).toBe(true)
    }
  })

  it('keeps Node and Electron out of the renderer', async () => {
    const page = await launch()
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    const probe = await page.evaluate(() => ({
      require: typeof (window as unknown as { require?: unknown }).require,
      process: typeof (window as unknown as { process?: unknown }).process,
      ipc: typeof (window as unknown as { ipcRenderer?: unknown }).ipcRenderer,
      bridgeKeys: Object.keys((window as unknown as { library: object }).library).sort(),
    }))
    expect(probe.require).toBe('undefined')
    expect(probe.process).toBe('undefined')
    expect(probe.ipc).toBe('undefined')
    expect(probe.bridgeKeys).toEqual(['invoke', 'onNodeStatus', 'platform', 'previewShell', 'windowControl'])
  })

  it('blocks methods that are not on the allowlist, and talks to the node for those that are', async () => {
    const page = await launch()
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    const run = (method: string) =>
      page.evaluate(async (m) => {
        try {
          return { ok: await (window as unknown as { library: { invoke(m: string): Promise<unknown> } }).library.invoke(m) }
        } catch (e) {
          return { error: String((e as Error).message) }
        }
      }, method)
    expect(await run('sync.subscribe')).toEqual({ error: expect.stringContaining('Blocked') })
    expect(await run('fs.readFile')).toEqual({ error: expect.stringContaining('Blocked') })
    const info = (await run('node.info')) as { ok: { name: string; protocol: number } }
    expect(info.ok.name).toBe('library-node')
    expect(info.ok.protocol).toBe(1)
    expect(await run('session.state')).toEqual({ ok: { state: 'signed_out' } })
  })

  it('refuses to load remote content and to navigate away', async () => {
    const page = await launch()
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    const csp = await page.evaluate(() => document.querySelector('meta[http-equiv="Content-Security-Policy"]')?.getAttribute('content') ?? '')
    expect(csp).toContain("default-src 'none'")
    await page.evaluate(() => {
      window.location.href = 'https://example.com/'
    })
    await page.waitForTimeout(500)
    expect(page.url()).toContain('index.html')
  })

  it('shows the shell in preview mode, resizes panes and toggles them from the keyboard', async () => {
    const page = await launch({ LIBRARY_PREVIEW_SHELL: '1' })
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor()
    await page.screenshot({ path: join(shots, 'shell-dark.png') })
    const sidebar = page.getByRole('complementary', { name: /sidebar/ })
    const w = async () => (await sidebar.boundingBox())?.width ?? 0
    expect(Math.round(await w())).toBe(264)
    const sep = page.getByRole('separator', { name: 'Resize sidebar' })
    await sep.focus()
    await page.keyboard.press('ArrowRight')
    expect(Math.round(await w())).toBe(280)
    await page.keyboard.press('End')
    expect(Math.round(await w())).toBe(420)
    await page.keyboard.press('Control+b')
    await sidebar.waitFor({ state: 'detached' })
    await page.keyboard.press('Control+j')
    await page.getByRole('region', { name: 'Panel' }).waitFor()
    await page.screenshot({ path: join(shots, 'shell-panel.png') })
  })

  it('draws window buttons that really work on Linux, and refuses other window actions', async () => {
    if (process.platform !== 'linux') return
    const page = await launch({ LIBRARY_PREVIEW_SHELL: '1' })
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor()
    const blocked = await page.evaluate(async () => {
      try {
        await (window as unknown as { library: { windowControl(a: string): Promise<void> } }).library.windowControl('quit')
        return 'allowed'
      } catch (e) {
        return String((e as Error).message)
      }
    })
    expect(blocked).toContain('Blocked')
    await page.screenshot({ path: join(shots, 'shell-controls.png'), clip: { x: 1200, y: 0, width: 240, height: 44 } })
    await page.getByRole('button', { name: 'Maximise' }).click()
    await page.getByRole('button', { name: 'Close' }).click()
    await app!.waitForEvent('close', { timeout: 10_000 })
  })

  it('applies the requested theme', async () => {
    const page = await launch()
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor)
    expect(bg).toBe('rgb(9, 9, 11)') // Ice Dark ground #09090b
  })
})
