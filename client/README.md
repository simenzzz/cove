# Cove Client

SvelteKit frontend for Cove.

## Development

Install dependencies and start the dev server:

```sh
npm install
npm run dev
```

The dev server runs on Vite's default port unless another port is already in
use.

## Checks

```sh
npm run check
npm test
npm run build
```

## Structure

- `src/routes/(auth)` - login and registration.
- `src/routes/(app)` - authenticated Cove workspace routes.
- `src/lib/components` - reusable UI and feature components.
- `src/lib/stores` - Svelte stores used as the frontend source of truth.
- `src/lib/ws` - WebSocket client and event bridge.
- `src/lib/api` - REST client.

The app expects the backend API and WebSocket endpoint from
`PUBLIC_API_URL` and `PUBLIC_WS_URL`, or same-origin `/api` and `/ws` in
production.
