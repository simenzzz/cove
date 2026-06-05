# Hardening Notes

This document captures security-relevant improvements that were made during
the production-readiness pass, plus low-priority follow-up items.

## Shipped

### Config and Secret Hygiene

- `NEXUS_ENV` separates development and production behavior.
- Production startup fails unless:
  - `JWT_SECRET` is at least 32 characters and not the example placeholder.
  - `SURREAL_USER` is not `root`.
  - `SURREAL_PASS` is not `root` and is at least 16 characters.
  - `SECURE_COOKIES=true`.
  - `CORS_ORIGIN` is explicitly set.
- JWT test secrets are compiled only for tests.
- Startup logs include namespace/database names, not full connection URLs.

### HTTP Surface

- Security headers are applied to every response:
  `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`,
  `Permissions-Policy`, and `Content-Security-Policy`.
- Global request body limit is 1 MB.
- CORS preflight cache uses `Access-Control-Max-Age: 3600`.
- Request IDs are applied at the top level so health and readiness routes are
  traceable too.

### Auth

- WebSocket ticket TTL is 10 seconds.
- WebSocket tickets bind a per-issue nonce that the client echoes on `/ws`.
- Tickets and nonces are consumed atomically from Redis.
- Login and register endpoints have per-IP rate limits.
- Refresh and logout use double-submit CSRF protection.
- Production refresh and CSRF cookies use the `__Host-` prefix.

### WebSocket Input

- Watch queue titles are rejected at the connection boundary when too long.
- Awareness update payloads are size-capped on whiteboard and post
  collaboration paths.
- Heartbeat handling enforces a server-side minimum interval.

### Error Responses

- Database, Redis, and internal errors return a fixed public message.
- Operators can correlate logs through the `x-request-id` response header.

### Infrastructure

- Redis requires AUTH.
- Compose interpolation fails fast for required production secrets.
- Caddy request body limits match the backend request body limit.
- Request-ID middleware wraps tracing so spans include the correlation ID.

## Follow-Up

These are intentionally small, isolated follow-ups.

1. Reduce refresh-token TTL from 7 days to 3 days if the product does not need
   week-long sessions.
2. Add cross-tab logout with `BroadcastChannel`.
3. Include `request_id` in JSON error bodies in addition to the response
   header.
4. Revisit `/api/auth/me` invalidation once profile edit endpoints exist.
