import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './App'
import { pickTheme } from './state/theme'
import './styles/base.css'
import './styles/shell.css'

let saved: string | null = null
try {
  saved = localStorage.getItem('library.theme')
} catch {
  // Storage can be unavailable; the default theme applies.
}
document.documentElement.dataset['theme'] = pickTheme(window.location.search, saved)

document.documentElement.dataset['platform'] = window.library.platform

const root = document.getElementById('root')
if (!root) throw new Error('missing #root')
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
