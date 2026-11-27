import { describe, expect, it } from 'vitest'
import { APP_METHODS, REMOTE_METHODS } from '../../src/generated/app-methods'
import { WINDOW_ACTIONS, checkInvoke, isWindowAction } from '../../src/shared/bridge'

describe('bridge allowlist', () => {
  it('allows every app-to-node method and nothing else', () => {
    for (const m of APP_METHODS) expect(checkInvoke(m, {}).ok, m).toBe(true)
    for (const m of REMOTE_METHODS) expect(checkInvoke(m, {}).ok, m).toBe(false)
    for (const m of ['', 'fs.readFile', 'shell.exec', '__proto__', 'node.info ', 'NODE.INFO']) expect(checkInvoke(m, {}).ok, m).toBe(false)
    expect(APP_METHODS).toContain('node.info')
    expect(APP_METHODS).not.toContain('sync.subscribe')
  })

  it('rejects non-string methods', () => {
    for (const m of [null, undefined, 42, {}, ['node.info'], Symbol.for('x')]) expect(checkInvoke(m, {}).ok).toBe(false)
  })

  it('accepts plain objects only as params', () => {
    expect(checkInvoke('node.info', undefined).ok).toBe(true)
    expect(checkInvoke('node.info', { a: 1 }).ok).toBe(true)
    for (const p of [[], 'x', 5, new Date(), new Map(), Object.create({ inherited: 1 })]) expect(checkInvoke('node.info', p).ok).toBe(false)
  })

  it('rejects params that are circular or too large', () => {
    const circular: Record<string, unknown> = {}
    circular['self'] = circular
    expect(checkInvoke('node.info', circular).ok).toBe(false)
    expect(checkInvoke('node.info', { blob: 'x'.repeat(300_000) }).ok).toBe(false)
  })
})

describe('window actions', () => {
  it('allows exactly minimise, maximise and close', () => {
    for (const a of WINDOW_ACTIONS) expect(isWindowAction(a)).toBe(true)
    for (const a of ['quit', 'reload', 'devtools', '', null, 1, {}]) expect(isWindowAction(a)).toBe(false)
  })
})
