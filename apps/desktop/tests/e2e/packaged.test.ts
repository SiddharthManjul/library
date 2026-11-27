// Smoke test of the installed layout: run the unpacked package (`pnpm package:dir`).
// An installed app ignores LIBRARY_NODE_PATH and must find the node bundled in its resources.
// Playwright's own launcher passes `--inspect`, which the hardened build refuses (fuse
// EnableNodeCliInspectArguments is off), so we start the app ourselves and attach over CDP.
import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { chromium } from 'playwright-core'
import { afterEach, describe, expect, it } from 'vitest'

const exe = process.env['LIBRARY_PACKAGED_APP'] ?? resolve(__dirname, '../../dist/linux-unpacked/library')
let port = 0
let proc: ChildProcess | null = null
const freshPort = () => (port = 9400 + Math.floor(Math.random() * 500))
afterEach(async () => {
  const p = proc
  proc = null
  if (!p || p.exitCode !== null) return
  // Wait for the process to be gone, so the next test starts clean (single-instance lock, ports).
  await new Promise<void>((done) => {
    p.once('exit', () => done())
    p.kill()
    setTimeout(done, 5000)
  })
})

async function waitForCdp(): Promise<void> {
  for (let i = 0; i < 60; i++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/json/version`)).ok) return
    } catch {
      // not up yet
    }
    await new Promise((r) => setTimeout(r, 250))
  }
  throw new Error('the packaged app did not open its debugging port')
}

describe.skipIf(!existsSync(exe))('packaged app', () => {
  it('finds its bundled node, ignores node overrides and reaches the sign-in screen', async () => {
    freshPort()
    proc = spawn(exe, ['--no-sandbox', `--remote-debugging-port=${port}`], { env: { ...process.env, LIBRARY_NODE_PATH: '/nonexistent/evil' }, stdio: 'ignore' })
    await waitForCdp()
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`)
    const page = browser.contexts()[0]!.pages()[0]!
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor()
    await page.getByText(/Local node ready/).waitFor()
    const hasBridge = await page.evaluate(() => typeof (window as unknown as { library?: unknown }).library)
    expect(hasBridge).toBe('object')
    await browser.close()
  })

  it('ignores the node inspector flag (the Electron fuse is on)', async () => {
    // With the fuse off, `--inspect` would open a Node debugger on 9229 that can run any code in the main process.
    freshPort()
    proc = spawn(exe, ['--no-sandbox', '--inspect=9229', `--remote-debugging-port=${port}`], { stdio: 'ignore' })
    await waitForCdp()
    const open = await fetch('http://127.0.0.1:9229/json/version').then(
      (r) => r.ok,
      () => false,
    )
    expect(open).toBe(false)
  })
})
