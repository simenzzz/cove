// Test stub for SvelteKit's `$env/dynamic/public` virtual module.
// A fixed WS URL keeps `websocketUrl()` deterministic in the client suite.
export const env: Record<string, string> = {
  PUBLIC_API_URL: '',
  PUBLIC_WS_URL: 'ws://localhost:3001/ws',
};
