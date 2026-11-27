import { Icon, type IconName } from '../icons/Icon'

export interface View {
  id: string
  label: string
  icon: IconName
}

export const TOP_VIEWS: View[] = [
  { id: 'explorer', label: 'Explorer', icon: 'files' },
  { id: 'search', label: 'Search', icon: 'search' },
  { id: 'source-control', label: 'Source control', icon: 'branch' },
  { id: 'run', label: 'Run and debug', icon: 'bug' },
  { id: 'extensions', label: 'Extensions', icon: 'extensions' },
  { id: 'testing', label: 'Testing', icon: 'flask' },
]
export const COMPANY_VIEWS: View[] = [
  { id: 'brain', label: 'Brain', icon: 'brain' },
  { id: 'graph', label: 'Graph', icon: 'graph' },
  { id: 'tasks', label: 'Tasks', icon: 'tasks' },
  { id: 'plugins', label: 'Plugins', icon: 'plug' },
]
export const BOTTOM_VIEWS: View[] = [
  { id: 'account', label: 'Account', icon: 'user' },
  { id: 'settings', label: 'Settings', icon: 'settings' },
]
export const ALL_VIEWS = [...TOP_VIEWS, ...COMPANY_VIEWS, ...BOTTOM_VIEWS]

interface Props {
  active: string
  onSelect(id: string): void
}

export function ActivityBar({ active, onSelect }: Props) {
  const item = (v: View) => (
    <button key={v.id} type="button" className="activity-btn" aria-label={v.label} title={v.label} aria-current={active === v.id ? 'page' : undefined} onClick={() => onSelect(v.id)}>
      <Icon name={v.icon} size={22} />
    </button>
  )
  return (
    <nav className="activity pane" aria-label="Activity bar">
      <div className="activity-group">{TOP_VIEWS.map(item)}</div>
      <div className="activity-sep" aria-hidden="true" />
      <div className="activity-group">{COMPANY_VIEWS.map(item)}</div>
      <div className="activity-group activity-bottom">{BOTTOM_VIEWS.map(item)}</div>
    </nav>
  )
}
