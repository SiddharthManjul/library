import type { ReactNode } from 'react'

/** Line icons on a 24 unit grid, 1.6 stroke, round caps and joins (BRAND.md section 6). No filled icons. */
const PATHS = {
  files: <><path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4" /></>,
  search: <><circle cx="11" cy="11" r="6.5" /><path d="m16 16 4.5 4.5" /></>,
  branch: <><circle cx="7" cy="6" r="2.2" /><circle cx="7" cy="18" r="2.2" /><circle cx="17" cy="9" r="2.2" /><path d="M7 8.2v7.6M17 11.2c0 3.4-4 3.4-9 5.2" /></>,
  bug: <><rect x="8" y="8" width="8" height="11" rx="4" /><path d="M8 12H4M16 12h4M8 16H5M16 16h3M10 5 8.5 3.5M14 5l1.5-1.5M12 8v11" /></>,
  extensions: <><rect x="4" y="4" width="7" height="7" rx="1" /><rect x="4" y="13" width="7" height="7" rx="1" /><rect x="13" y="13" width="7" height="7" rx="1" /><path d="M16.5 3.5v7M13 7h7" /></>,
  flask: <><path d="M9 3h6M10 3v6l-5 9a2 2 0 0 0 1.8 3h10.4a2 2 0 0 0 1.8-3l-5-9V3" /><path d="M7.5 15h9" /></>,
  brain: <><path d="M12 5v14" /><path d="M12 7a3 3 0 0 0-5.5-1.5A3.5 3.5 0 0 0 5 12a3.5 3.5 0 0 0 1.5 6A3 3 0 0 0 12 17" /><path d="M12 7a3 3 0 0 1 5.5-1.5A3.5 3.5 0 0 1 19 12a3.5 3.5 0 0 1-1.5 6A3 3 0 0 1 12 17" /></>,
  graph: <><circle cx="6" cy="6" r="2.2" /><circle cx="18" cy="8" r="2.2" /><circle cx="9" cy="18" r="2.2" /><circle cx="18" cy="18" r="2.2" /><path d="m8 6.3 8 1.4M7 8l1.5 8M11.2 18h4.6M17.8 10.2v5.6" /></>,
  tasks: <><rect x="4" y="4" width="16" height="16" rx="2" /><path d="m8.5 12 2.5 2.5 4.5-5" /></>,
  plug: <path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4" />,
  user: <><circle cx="12" cy="8" r="3.5" /><path d="M5 20a7 7 0 0 1 14 0" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M18.4 5.6l-1.8 1.8M7.4 16.6l-1.8 1.8" /></>,
  bell: <path d="M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15zM10 20.5a2 2 0 0 0 4 0" />,
  more: <><circle cx="12" cy="5.5" r=".9" /><circle cx="12" cy="12" r=".9" /><circle cx="12" cy="18.5" r=".9" /></>,
  panelLeft: <><rect x="3.5" y="4.5" width="17" height="15" rx="2" /><path d="M9.5 4.5v15" /></>,
  panelBottom: <><rect x="3.5" y="4.5" width="17" height="15" rx="2" /><path d="M3.5 14.5h17" /></>,
  panelRight: <><rect x="3.5" y="4.5" width="17" height="15" rx="2" /><path d="M14.5 4.5v15" /></>,
  chevronDown: <path d="m6 9 6 6 6-6" />,
  terminal: <path d="m5 8 4 4-4 4M12 17h7" />,
  alert: <path d="M12 4 21 19H3zM12 10v4M12 16.5v.5" />,
  lock: <><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></>,
  chat: <path d="M5 5h14v10h-9l-5 4z" />,
  check: <><circle cx="12" cy="12" r="9" /><path d="m8 12 3 3 5-6" /></>,
} satisfies Record<string, ReactNode>

export type IconName = keyof typeof PATHS

export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      {PATHS[name]}
    </svg>
  )
}

/** The Library mark ("big corner arrow"), one colour, inheriting the text colour. */
export function Mark({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="6 6 36 36" fill="currentColor" aria-hidden="true" focusable="false">
      <path d="M7 7H17.5V30.5H41V41H7ZM21 7H25V10.5H21ZM29 7H41V10.5H29ZM37.5 10.5H41V19H37.5ZM37.5 23H41V27H37.5ZM21.7626 23.7626L32.2626 13.2626L34.7374 15.7374L24.2374 26.2374Z" />
    </svg>
  )
}
