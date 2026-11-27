// Design rules from the brand kit, enforced: components use tokens, never raw colours or sizes.
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const src = resolve(__dirname, '../../src/renderer')
const files = (dir: string): string[] =>
  readdirSync(dir).flatMap((f) => {
    const p = join(dir, f)
    return statSync(p).isDirectory() ? files(p) : [p]
  })
const code = files(src).filter((f) => /\.(css|tsx?)$/.test(f))

describe('design rules', () => {
  it('has no raw colours in renderer code', () => {
    for (const f of code) {
      const text = readFileSync(f, 'utf8')
      expect(text, f).not.toMatch(/#[0-9a-fA-F]{3,8}\b/)
      expect(text, f).not.toMatch(/\brgba?\(/)
      expect(text, f).not.toMatch(/\bhsla?\(/)
    }
  })

  it('has no fonts outside the three brand families', () => {
    for (const f of code.filter((c) => c.endsWith('.css'))) {
      const text = readFileSync(f, 'utf8').replace(/@import[^;]+;/g, '')
      for (const m of text.matchAll(/font-family:\s*([^;]+);/g)) expect(m[1], f).toMatch(/var\(--font-(sans|display|mono)\)/)
    }
  })

  it('uses only the brand radii', () => {
    for (const f of code.filter((c) => c.endsWith('.css'))) {
      for (const m of readFileSync(f, 'utf8').matchAll(/border-radius:\s*([^;]+);/g)) expect(m[1], f).toMatch(/var\(--radius-(sm|md|lg)\)|^50%$|^0$|^1px$/)
    }
  })
})
