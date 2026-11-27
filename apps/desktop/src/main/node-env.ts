/**
 * The environment the local node starts with. Nothing is inherited by default: the node holds the member's
 * session, so it gets only what it needs to find its keychain, reach the network (including a company
 * proxy) and write temporary files.
 */

/** What every build passes through when it is set. */
const ALWAYS = [
  'PATH',
  // Where the user's own data and the OS keychain live.
  'HOME', 'USER', 'LOGNAME', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS',
  'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'SystemRoot', 'SYSTEMROOT', 'ComSpec', 'TEMP', 'TMP', 'TMPDIR',
  // Company proxies, so the node's connection works on a managed network.
  'HTTPS_PROXY', 'HTTP_PROXY', 'NO_PROXY', 'ALL_PROXY', 'https_proxy', 'http_proxy', 'no_proxy', 'all_proxy',
] as const

/**
 * Passed only by development builds. An installed app must not let an environment variable point the node at
 * another server or at a plaintext secret file.
 */
const DEV_ONLY = ['LIBRARY_GATEWAY_URL', 'LIBRARY_DEV_KEYCHAIN_FILE', 'RUST_LOG'] as const

export function nodeEnv(source: Readonly<Record<string, string | undefined>>, isPackaged: boolean): Record<string, string> {
  const names: readonly string[] = isPackaged ? ALWAYS : [...ALWAYS, ...DEV_ONLY]
  const out: Record<string, string> = {}
  for (const name of names) {
    const v = source[name]
    if (v !== undefined) out[name] = v
  }
  return out
}
