import { defineConfig } from 'vitest/config'

// The first Electron launch on a fresh machine can take over a minute (antivirus scanning new files), so the
// limits are generous. Warm runs finish in seconds.
export default defineConfig({
  test: { globals: true, environment: 'node', include: ['tests/e2e/**/*.test.ts'], testTimeout: 180_000, hookTimeout: 180_000, fileParallelism: false },
})
