import { describe, expect, it } from 'vitest'
import { resolveNodeBinary } from '../../src/main/node-path'

const base = { resourcesPath: '/res', appPath: '/repo/apps/desktop', override: '/tmp/evil' }

describe('node binary location', () => {
  it('uses the bundled binary when installed, and ignores any override', () => {
    expect(resolveNodeBinary({ ...base, isPackaged: true, platform: 'linux' })).toBe('/res/bin/library-node')
    expect(resolveNodeBinary({ ...base, isPackaged: true, platform: 'win32' })).toMatch(/library-node\.exe$/)
  })

  it('honours the override only in development', () => {
    expect(resolveNodeBinary({ ...base, isPackaged: false, platform: 'linux' })).toBe('/tmp/evil')
    expect(resolveNodeBinary({ ...base, isPackaged: false, platform: 'linux', override: undefined })).toBe('/repo/target/debug/library-node')
  })
})
