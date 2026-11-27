import { useRef, type KeyboardEvent, type PointerEvent } from 'react'
import { LIMITS, type PaneId } from '../state/layout'

const STEP = 16

interface Props {
  pane: PaneId
  label: string
  size: number
  /** `x` panes grow when dragged right, unless `grow` says they grow leftwards or upwards. */
  axis: 'x' | 'y'
  grow: 1 | -1
  onResize(size: number): void
}

/** The 4px gap between two panes. Drag it, or focus it and use the arrow keys. */
export function Resizer({ pane, label, size, axis, grow, onResize }: Props) {
  const start = useRef<{ at: number; size: number } | null>(null)
  const { min, max } = LIMITS[pane]
  const coord = (e: { clientX: number; clientY: number }) => (axis === 'x' ? e.clientX : e.clientY)

  const down = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId)
    start.current = { at: coord(e), size }
  }
  const move = (e: PointerEvent<HTMLDivElement>) => {
    if (start.current) onResize(start.current.size + (coord(e) - start.current.at) * grow)
  }
  const up = (e: PointerEvent<HTMLDivElement>) => {
    start.current = null
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId)
  }
  const key = (e: KeyboardEvent<HTMLDivElement>) => {
    const more = axis === 'x' ? (grow === 1 ? 'ArrowRight' : 'ArrowLeft') : grow === 1 ? 'ArrowDown' : 'ArrowUp'
    const less = axis === 'x' ? (grow === 1 ? 'ArrowLeft' : 'ArrowRight') : grow === 1 ? 'ArrowUp' : 'ArrowDown'
    if (e.key === more) onResize(size + STEP)
    else if (e.key === less) onResize(size - STEP)
    else if (e.key === 'Home') onResize(min)
    else if (e.key === 'End') onResize(max)
    else return
    e.preventDefault()
  }

  return (
    <div
      className={`resizer resizer-${axis}`}
      role="separator"
      aria-orientation={axis === 'x' ? 'vertical' : 'horizontal'}
      aria-label={label}
      aria-valuenow={size}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
      onKeyDown={key}
    />
  )
}
