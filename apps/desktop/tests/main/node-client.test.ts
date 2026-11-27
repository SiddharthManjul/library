import { EventEmitter } from 'node:events'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { NodeClient, RpcFailure, type ChildLike } from '../../src/main/node-client'

class FakeChild extends EventEmitter implements ChildLike {
  written: string[] = []
  killed = false
  stdout = new EventEmitter() as unknown as ChildLike['stdout']
  stderr = new EventEmitter() as unknown as ChildLike['stderr']
  stdin = { write: (c: string) => (this.written.push(c), true), end: () => undefined }
  kill() {
    this.killed = true
    return true
  }
  say(obj: unknown) {
    ;(this.stdout as unknown as EventEmitter).emit('data', JSON.stringify(obj) + '\n')
  }
  lastId() {
    return JSON.parse(this.written.at(-1) ?? '{}').id as number
  }
}

let children: FakeChild[]
const make = () => {
  const c = new FakeChild()
  children.push(c)
  return c
}
const ok = (id: number, result: unknown = {}) => ({ jsonrpc: '2.0', id, result, error: null })

beforeEach(() => {
  children = []
  vi.useFakeTimers()
})
afterEach(() => vi.useRealTimers())

describe('NodeClient', () => {
  it('matches responses to requests by id, in any order', async () => {
    const c = new NodeClient(make)
    c.start()
    const a = c.request('local.ask')
    const b = c.request('local.runs')
    const [, ra, rb] = children[0]!.written.map((w) => JSON.parse(w) as { id: number })
    children[0]!.say(ok(rb!.id, 'second'))
    children[0]!.say(ok(ra!.id, 'first'))
    await expect(a).resolves.toBe('first')
    await expect(b).resolves.toBe('second')
    c.stop()
  })

  it('becomes ready when the node answers node.info', async () => {
    const c = new NodeClient(make)
    const seen: string[] = []
    c.on('status', (s: string) => seen.push(s))
    c.start()
    children[0]!.say(ok(1, { name: 'library-node' }))
    await vi.advanceTimersByTimeAsync(0)
    expect(c.getStatus()).toBe('ready')
    expect(seen).toEqual(['starting', 'ready'])
    c.stop()
  })

  it('turns an rpc error into a typed failure', async () => {
    const c = new NodeClient(make)
    c.start()
    const p = c.request('local.ask')
    children[0]!.say({ jsonrpc: '2.0', id: children[0]!.lastId(), result: null, error: { code: -32001, message: 'Sign in to continue.', data: null } })
    await expect(p).rejects.toBeInstanceOf(RpcFailure)
    c.stop()
  })

  it('times out a request that is never answered', async () => {
    const c = new NodeClient(make, { timeoutMs: 1000 })
    c.start()
    const p = c.request('local.ask')
    const assertion = expect(p).rejects.toThrow(/in time/)
    await vi.advanceTimersByTimeAsync(1001)
    await assertion
    c.stop()
  })

  it('fails pending requests and restarts when the node exits', async () => {
    const c = new NodeClient(make, { backoffMs: 100 })
    c.start()
    const p = c.request('local.ask')
    const assertion = expect(p).rejects.toThrow(/unexpectedly/)
    children[0]!.emit('exit')
    await assertion
    expect(c.getStatus()).toBe('restarting')
    await vi.advanceTimersByTimeAsync(101)
    expect(children).toHaveLength(2)
    c.stop()
  })

  it('gives up after a crash loop instead of restarting forever', async () => {
    const c = new NodeClient(make, { backoffMs: 10, maxRestarts: 3 })
    c.start()
    for (let i = 0; i < 6; i++) {
      children.at(-1)?.emit('exit')
      await vi.advanceTimersByTimeAsync(1000)
    }
    expect(c.getStatus()).toBe('failed')
    expect(children.length).toBeLessThanOrEqual(4)
    await expect(c.request('node.info')).rejects.toThrow(/not running/)
  })

  it('kills a node that sends an oversized line', async () => {
    const c = new NodeClient(make, { maxLineBytes: 100 })
    c.start()
    ;(children[0]!.stdout as unknown as EventEmitter).emit('data', 'x'.repeat(500))
    expect(children[0]!.killed).toBe(true)
    c.stop()
  })

  it('ignores garbage and unknown ids', async () => {
    const c = new NodeClient(make)
    c.start()
    const p = c.request('local.ask')
    ;(children[0]!.stdout as unknown as EventEmitter).emit('data', 'not json\n')
    children[0]!.say(ok(9999))
    children[0]!.say(ok(children[0]!.lastId(), 'real'))
    await expect(p).resolves.toBe('real')
    c.stop()
  })

  it('never writes after stop', async () => {
    const c = new NodeClient(make)
    c.start()
    c.stop()
    await expect(c.request('node.info')).rejects.toThrow()
  })
})
