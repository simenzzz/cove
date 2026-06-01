# Cove

Cove is a Discord-style social workspace built around real-time chat,
collaborative documents, shared whiteboards, watch rooms, and graph-based
discovery.

The project is intentionally full stack: a Rust/Axum backend, a SvelteKit
frontend, SurrealDB for document and graph data, Redis for ephemeral state, and
WebSockets for live collaboration.

## What It Shows

- Real-time chat with typed WebSocket protocol messages, resumable streams, and
  room actors.
- Social graph features using SurrealDB edges for friends, follows,
  membership, discovery, and recommendations.
- Collaborative editing and whiteboards backed by CRDT data structures.
- Watch rooms with synchronized playback, queues, voting, and reactions.
- Production-oriented auth: access tokens, refresh cookies, CSRF protection,
  single-use WebSocket tickets, rate limiting, and security headers.
- Docker-based local and production deployments with Caddy, Redis, and
  SurrealDB.

## Stack

| Layer | Technology |
| --- | --- |
| Backend | Rust, Axum, Tokio |
| Frontend | SvelteKit, TypeScript |
| Database | SurrealDB |
| Ephemeral state | Redis |
| Collaboration | Yrs/Yjs CRDTs |
| Deployment | Docker Compose, Caddy |

## Repository Layout

```text
server/       Rust backend
client/       SvelteKit frontend
db/           SurrealDB bootstrap schema
docs/         Setup and architecture docs
ProjectDocs/  Design notes and protocol specs
roadmaps/     Product roadmap
```

## Local Development

```bash
git clone https://github.com/simenzzz/cove.git
cd cove
cp .env.example .env
docker compose up --build
```

Once the stack is running:

- Frontend: http://localhost:3000
- Backend API: http://localhost:3001
- Health check: http://localhost:3001/health
- Readiness check: http://localhost:3001/ready

For hot-reload development, see [docs/setup.md](docs/setup.md).

## Verification

```bash
cd server
cargo test
```

```bash
cd client
npm install
npm run check
npm test
```

## Design Notes

- [Architecture overview](docs/architecture.md)
- [Setup guide](docs/setup.md)
- [WebSocket protocol v1](ProjectDocs/ws-protocol-v1.md)
- [Auth design](ProjectDocs/auth-design.md)
- [Hardening notes](ProjectDocs/hardening.md)
