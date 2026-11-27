import { EventEmitter } from 'node:events'
import type { RpcResponse } from '../generated/protocol/RpcResponse'

/** The slice of a child process the client needs, so tests can supply a fake. */
export interface ChildLike {
  stdin: { write(chunk: string): boolean; end(): void }
  stdout: { on(event: 'data', cb: (chunk: Buffer | string) => void): unknown }
  stderr: { on(event: 'data', cb: (chunk: Buffer | string) => void): unknown } | null
  on(event: 'exit' | 'error', cb: (...args: unknown[]) => void): unknown
  kill(): boolean
}

export type NodeStatus = 'starting' | 'ready' | 'restarting' | 'failed' | 'stopped'

export class RpcFailure extends Error {
  constructor(
    public readonly code: number,
    message: string,
    public readonly data: unknown = null,
  ) {
    super(message)
    this.name = 'RpcFailure'
  }
}

export interface NodeClientOptions {
  timeoutMs?: number
  /** A node line longer than this is a protocol violation and the node is restarted. */
  maxLineBytes?: number
  maxRestarts?: number
  restartWindowMs?: number
  backoffMs?: number
}

interface Pending {
  resolve: (v: unknown) => void
  reject: (e: Error) => void
  timer: ReturnType<typeof setTimeout>
}

/**
 * Speaks line-delimited JSON-RPC to the local node over stdio. Restarts the node when it exits
 * (with backoff, and a cap so a crash loop ends in `failed`). Requests never outlive their timeout.
 * Params and results are never logged.
 */
export class NodeClient extends EventEmitter {
  private child: ChildLike | null = null
  private nextId = 1
  private pending = new Map<number, Pending>()
  private buffer = ''
  private restarts: number[] = []
  private stopped = false
  private status: NodeStatus = 'stopped'
  private readonly opt: Required<NodeClientOptions>

  constructor(
    private readonly spawn: () => ChildLike,
    options: NodeClientOptions = {},
  ) {
    super()
    this.opt = { timeoutMs: 10_000, maxLineBytes: 1024 * 1024, maxRestarts: 5, restartWindowMs: 60_000, backoffMs: 250, ...options }
  }

  getStatus(): NodeStatus {
    return this.status
  }

  start(): void {
    this.stopped = false
    this.launch('starting')
  }

  stop(): void {
    this.stopped = true
    this.setStatus('stopped')
    this.failAll(new Error('The local node stopped.'))
    try {
      this.child?.stdin.end()
      this.child?.kill()
    } catch {
      // The process may already be gone.
    }
    this.child = null
  }

  request(method: string, params: unknown = {}): Promise<unknown> {
    const child = this.child
    if (!child || this.status === 'failed' || this.status === 'stopped') {
      return Promise.reject(new Error('The local node is not running.'))
    }
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error('The local node did not answer in time.'))
      }, this.opt.timeoutMs)
      this.pending.set(id, { resolve, reject, timer })
      try {
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
      } catch {
        clearTimeout(timer)
        this.pending.delete(id)
        reject(new Error('The local node is not reachable.'))
      }
    })
  }

  private launch(status: NodeStatus): void {
    this.buffer = ''
    this.setStatus(status)
    let child: ChildLike
    try {
      child = this.spawn()
    } catch {
      this.onExit()
      return
    }
    this.child = child
    child.stdout.on('data', (chunk) => this.onData(child, chunk.toString()))
    // stderr carries diagnostics only; it is drained so the pipe never fills, and never forwarded.
    child.stderr?.on('data', () => undefined)
    child.on('error', () => this.onExit(child))
    child.on('exit', () => this.onExit(child))
    // The node answers `node.info` as soon as it is up, which is how we know it is ready.
    this.request('node.info').then(
      () => child === this.child && this.setStatus('ready'),
      () => undefined,
    )
  }

  private onData(child: ChildLike, text: string): void {
    if (child !== this.child) return
    this.buffer += text
    let nl = this.buffer.indexOf('\n')
    while (nl >= 0) {
      const line = this.buffer.slice(0, nl)
      this.buffer = this.buffer.slice(nl + 1)
      this.onLine(line)
      nl = this.buffer.indexOf('\n')
    }
    if (Buffer.byteLength(this.buffer) > this.opt.maxLineBytes) {
      this.buffer = ''
      try {
        child.kill()
      } catch {
        // Already gone.
      }
    }
  }

  private onLine(line: string): void {
    let msg: RpcResponse
    try {
      msg = JSON.parse(line) as RpcResponse
    } catch {
      return
    }
    const p = typeof msg.id === 'number' ? this.pending.get(msg.id) : undefined
    if (!p) return
    this.pending.delete(msg.id)
    clearTimeout(p.timer)
    if (msg.error) p.reject(new RpcFailure(msg.error.code, msg.error.message, msg.error.data))
    else p.resolve(msg.result)
  }

  private onExit(child?: ChildLike): void {
    if (child && child !== this.child) return
    this.child = null
    this.failAll(new Error('The local node stopped unexpectedly.'))
    if (this.stopped) return
    const now = Date.now()
    this.restarts = this.restarts.filter((t) => now - t < this.opt.restartWindowMs)
    if (this.restarts.length >= this.opt.maxRestarts) {
      this.setStatus('failed')
      return
    }
    const delay = this.opt.backoffMs * 2 ** this.restarts.length
    this.restarts.push(now)
    this.setStatus('restarting')
    setTimeout(() => !this.stopped && this.launch('restarting'), delay)
  }

  private failAll(err: Error): void {
    for (const [id, p] of this.pending) {
      clearTimeout(p.timer)
      p.reject(err)
      this.pending.delete(id)
    }
  }

  private setStatus(s: NodeStatus): void {
    if (this.status === s) return
    this.status = s
    this.emit('status', s)
  }
}
