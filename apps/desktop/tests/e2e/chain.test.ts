// The whole chain with nothing mocked: renderer -> preload -> Electron main -> the real node binary ->
// the real gateway over a real WebSocket -> Postgres. Needs the dev Postgres, a display, and
// `cargo build -p library-node -p library-server --examples`.
import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { _electron as electron, type ElectronApplication, type Page } from 'playwright-core'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

const root = resolve(__dirname, '../..')
const target = resolve(root, '../../target/debug')
const nodeBin = join(target, process.platform === 'win32' ? 'library-node.exe' : 'library-node')
const gatewayBin = join(target, 'examples', process.platform === 'win32' ? 'e2e_gateway.exe' : 'e2e_gateway')

interface Seed {
  url: string
  tenant_id: string
  user_id: string
  access: string
  refresh: string
  access_expires_at: number
  foreign_scope: string
}

let gateway: ChildProcess | null = null
let app: ElectronApplication | null = null
let dir = ''

async function startGateway(): Promise<Seed> {
  gateway = spawn(gatewayBin, [], { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, DATABASE_ADMIN_URL: process.env['DATABASE_ADMIN_URL'] ?? 'postgres://postgres:postgres@127.0.0.1:5433/postgres' } })
  const child = gateway
  return await new Promise<Seed>((res, rej) => {
    const t = setTimeout(() => rej(new Error('the test gateway did not start (is the dev Postgres up?)')), 30_000)
    let buf = ''
    child.stdout?.on('data', (d: Buffer) => {
      buf += d.toString()
      const nl = buf.indexOf('\n')
      if (nl >= 0) {
        clearTimeout(t)
        res(JSON.parse(buf.slice(0, nl)) as Seed)
      }
    })
    child.on('exit', () => rej(new Error('the test gateway exited early')))
  })
}

/** The session exactly as the sign-in flow will store it. */
function seedKeychain(s: Seed, overrides: Partial<Seed> = {}): string {
  const file = join(dir, 'keychain.json')
  const session = { tenant_id: s.tenant_id, user_id: s.user_id, access: s.access, refresh: s.refresh, access_expires_at: s.access_expires_at, ...overrides }
  writeFileSync(file, JSON.stringify({ 'session-v1': JSON.stringify(session) }))
  return file
}

async function launch(s: Seed, keychainFile: string): Promise<Page> {
  app = await electron.launch({
    args: [join(root, 'out/main/index.js'), '--no-sandbox'],
    cwd: root,
    env: { ...process.env, LIBRARY_NODE_PATH: nodeBin, LIBRARY_GATEWAY_URL: s.url, LIBRARY_DEV_KEYCHAIN_FILE: keychainFile },
  })
  const page = await app.firstWindow()
  await expect.poll(() => app!.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.isVisible() ?? false), { timeout: 15_000 }).toBe(true)
  return page
}

const invoke = (page: Page, method: string, params: Record<string, unknown> = {}) =>
  page.evaluate(
    async ([m, p]) => {
      try {
        return { ok: await (window as unknown as { library: { invoke(m: string, p: unknown): Promise<unknown> } }).library.invoke(m as string, p) }
      } catch (e) {
        return { error: String((e as Error).message) }
      }
    },
    [method, params] as const,
  )

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), 'library-e2e-'))
})
afterEach(async () => {
  await app?.close().catch(() => undefined)
  app = null
  gateway?.stdin?.end()
  await new Promise((r) => setTimeout(r, 1500))
  gateway?.kill()
  gateway = null
  rmSync(dir, { recursive: true, force: true })
})

describe.skipIf(!existsSync(nodeBin) || !existsSync(gatewayBin))('desktop to gateway, nothing mocked', () => {
  it('shows the shell as signed in and reads who the server says the member is', async () => {
    const seed = await startGateway()
    const page = await launch(seed, seedKeychain(seed))
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor({ timeout: 20_000 })
    await page.getByText('Connected', { exact: true }).waitFor()

    const who = (await invoke(page, 'remote.call', { method: 'session.whoami' })) as { ok: { tenant_id: string; user_id: string; plan: string; scopes: string[] } }
    expect(who.ok.tenant_id).toBe(seed.tenant_id)
    expect(who.ok.user_id).toBe(seed.user_id)
    expect(who.ok.plan).toBe('starter')
    expect(who.ok.scopes).toHaveLength(2)
  })

  it('never lets a token or a secret reach the renderer', async () => {
    const seed = await startGateway()
    const page = await launch(seed, seedKeychain(seed))
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor({ timeout: 20_000 })
    const state = JSON.stringify(await invoke(page, 'session.state'))
    const everything = state + (await page.content()) + (await page.evaluate(() => JSON.stringify(Object.keys(window.library))))
    expect(everything).not.toContain(seed.access)
    expect(everything).not.toContain(seed.refresh)
    expect(everything).not.toMatch(/lrt1_|v1\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}/)
  })

  it('refuses what the server refuses, with the typed reason, and what the app may not call at all', async () => {
    const seed = await startGateway()
    const page = await launch(seed, seedKeychain(seed))
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor({ timeout: 20_000 })
    expect(await invoke(page, 'remote.call', { method: 'org.departments.upsert', params: {}, idempotency_key: 'e2e-key-0123456' })).toEqual({ error: expect.stringContaining('role') })
    expect(await invoke(page, 'remote.call', { method: 'sync.snapshot', params: { scope_id: seed.foreign_scope } })).toEqual({ error: expect.stringContaining('scope') })
    expect(await invoke(page, 'remote.call', { method: 'brain.context', params: { scope_id: seed.foreign_scope } })).toEqual({ error: expect.stringContaining('cannot call') })
    expect(await invoke(page, 'remote.call', { method: 'session.refresh', params: { refresh: seed.refresh } })).toEqual({ error: expect.stringContaining('cannot call') })
    expect(await invoke(page, 'remote.call', { method: 'fs.readFile' })).toEqual({ error: expect.stringContaining('does not exist') })
  })

  it('refreshes a stale session on its own and comes online', async () => {
    const seed = await startGateway()
    // The access token is dead, as after a weekend with the lid closed. The refresh token still works.
    const page = await launch(seed, seedKeychain(seed, { access: 'v1.deadDeadDeadDeadDead.deadDeadDeadDeadDead', access_expires_at: 1 }))
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor({ timeout: 20_000 })
    await page.getByText('Connected', { exact: true }).waitFor()
    const who = (await invoke(page, 'remote.call', { method: 'session.whoami' })) as { ok: { user_id: string } }
    expect(who.ok.user_id).toBe(seed.user_id)
  })

  it('signs the device out when the server refuses its session, and shows the sign-in screen', async () => {
    const seed = await startGateway()
    const keychain = seedKeychain(seed, { access: 'v1.deadDeadDeadDeadDead.deadDeadDeadDeadDead', access_expires_at: 1, refresh: 'lrt1_thisRefreshTokenWasNeverIssuedByAnyone' })
    const page = await launch(seed, keychain)
    await page.getByRole('heading', { name: 'Sign in to Library' }).waitFor({ timeout: 20_000 })
    expect(await invoke(page, 'remote.call', { method: 'session.whoami' })).toEqual({ error: expect.stringContaining('Sign in') })
  })

  it('works offline from what is stored and says so in words', async () => {
    const seed = await startGateway()
    const keychain = seedKeychain(seed)
    // Point the node at a port nothing listens on.
    const offline = { ...seed, url: 'http://127.0.0.1:9' }
    const page = await launch(offline, keychain)
    await page.getByRole('navigation', { name: 'Activity bar' }).waitFor({ timeout: 20_000 })
    await page.getByText('Offline, working from this computer').waitFor({ timeout: 20_000 })
    const r = (await invoke(page, 'remote.call', { method: 'session.whoami' })) as { error: string }
    expect(r.error).toContain('Cannot reach Library')
  })
})
