import { describe, expect, it } from 'vitest'
import { nodeEnv } from '../../src/main/node-env'

const parent = {
  PATH: '/usr/bin',
  HOME: '/home/ravi',
  DBUS_SESSION_BUS_ADDRESS: 'unix:path=/run/user/1000/bus',
  XDG_RUNTIME_DIR: '/run/user/1000',
  HTTPS_PROXY: 'http://proxy.corp:8080',
  SystemRoot: 'C:\\Windows',
  LIBRARY_GATEWAY_URL: 'http://127.0.0.1:9',
  LIBRARY_DEV_KEYCHAIN_FILE: '/tmp/secrets.json',
  AWS_SECRET_ACCESS_KEY: 'do-not-pass-me-on',
  GITHUB_TOKEN: 'do-not-pass-me-on',
  ANTHROPIC_API_KEY: 'do-not-pass-me-on',
  NODE_OPTIONS: '--require /evil.js',
  LD_PRELOAD: '/evil.so',
}

describe('node environment', () => {
  it('passes what the node needs to find its keychain and reach the network', () => {
    const env = nodeEnv(parent, true)
    expect(env['DBUS_SESSION_BUS_ADDRESS']).toBe('unix:path=/run/user/1000/bus') // without it, Linux has no Secret Service
    expect(env['XDG_RUNTIME_DIR']).toBe('/run/user/1000')
    expect(env['HTTPS_PROXY']).toBe('http://proxy.corp:8080')
    expect(env['SystemRoot']).toBe('C:\\Windows')
    expect(env['PATH']).toBe('/usr/bin')
  })

  it('never passes credentials or code-injection variables from the parent', () => {
    for (const packaged of [true, false]) {
      const env = nodeEnv(parent, packaged)
      expect(Object.keys(env)).not.toContain('AWS_SECRET_ACCESS_KEY')
      expect(Object.keys(env)).not.toContain('GITHUB_TOKEN')
      expect(Object.keys(env)).not.toContain('ANTHROPIC_API_KEY')
      expect(Object.keys(env)).not.toContain('NODE_OPTIONS')
      expect(Object.keys(env)).not.toContain('LD_PRELOAD')
      expect(JSON.stringify(env)).not.toContain('do-not-pass-me-on')
    }
  })

  it('lets an installed app ignore the development overrides, and a development build use them', () => {
    const installed = nodeEnv(parent, true)
    expect(installed).not.toHaveProperty('LIBRARY_GATEWAY_URL')
    expect(installed).not.toHaveProperty('LIBRARY_DEV_KEYCHAIN_FILE')
    const dev = nodeEnv(parent, false)
    expect(dev['LIBRARY_GATEWAY_URL']).toBe('http://127.0.0.1:9')
    expect(dev['LIBRARY_DEV_KEYCHAIN_FILE']).toBe('/tmp/secrets.json')
  })

  it('leaves out variables that are not set instead of passing "undefined"', () => {
    expect(nodeEnv({ PATH: '/bin' }, true)).toEqual({ PATH: '/bin' })
    expect(nodeEnv({}, true)).toEqual({})
  })
})
