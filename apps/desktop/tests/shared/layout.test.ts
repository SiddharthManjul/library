import { describe, expect, it } from 'vitest'
import { CLOSE_BELOW_MIN, DEFAULTS, LIMITS, initialLayout, layoutReducer } from '../../src/renderer/state/layout'
import { pickTheme } from '../../src/renderer/state/theme'

describe('layout', () => {
  it('starts from the brand token sizes', () => {
    expect(DEFAULTS).toEqual({ sidebar: 264, panel: 232, agent: 340 })
    expect(initialLayout.sidebar.size).toBe(264)
    expect(initialLayout.panel.open).toBe(false)
  })

  it('clamps a resize to the limits', () => {
    const big = layoutReducer(initialLayout, { type: 'resize', pane: 'sidebar', size: 9999 })
    expect(big.sidebar.size).toBe(LIMITS.sidebar.max)
    const small = layoutReducer(initialLayout, { type: 'resize', pane: 'agent', size: LIMITS.agent.min - 10 })
    expect(small.agent.size).toBe(LIMITS.agent.min)
    expect(small.agent.open).toBe(true)
  })

  it('closes a pane dragged far past its minimum and keeps its last size for reopening', () => {
    const sized = layoutReducer(initialLayout, { type: 'resize', pane: 'sidebar', size: 300 })
    const closed = layoutReducer(sized, { type: 'resize', pane: 'sidebar', size: LIMITS.sidebar.min - CLOSE_BELOW_MIN - 1 })
    expect(closed.sidebar.open).toBe(false)
    expect(layoutReducer(closed, { type: 'toggle', pane: 'sidebar' }).sidebar).toEqual({ open: true, size: 300 })
  })

  it('toggles panes independently', () => {
    const s = layoutReducer(initialLayout, { type: 'toggle', pane: 'panel' })
    expect(s.panel.open && s.sidebar.open && s.agent.open).toBe(true)
  })

  it('un-maximises when the panel closes', () => {
    let s = layoutReducer(initialLayout, { type: 'maximisePanel' })
    expect(s.panel).toMatchObject({ open: true, maximised: true })
    s = layoutReducer(s, { type: 'toggle', pane: 'panel' })
    expect(s.panel).toMatchObject({ open: false, maximised: false })
    s = layoutReducer(layoutReducer(initialLayout, { type: 'maximisePanel' }), { type: 'restorePanel' })
    expect(s.panel.maximised).toBe(false)
  })
})

describe('theme', () => {
  it('defaults to Ice Dark and accepts only known themes', () => {
    expect(pickTheme('', null)).toBe('dark')
    expect(pickTheme('?theme=light', null)).toBe('light')
    expect(pickTheme('', 'dim')).toBe('dim')
    expect(pickTheme('?theme=dim', 'light')).toBe('dim')
    for (const bad of ['neon', '<script>', '']) expect(pickTheme(`?theme=${bad}`, bad)).toBe('dark')
  })
})
