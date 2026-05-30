/**
 * Normalize a SurrealDB RecordId into its plain key string.
 *
 * SurrealDB serializes record IDs in several shapes depending on the codec:
 *   - "user:abc"                      (string form)
 *   - { tb: "user", id: "abc" }       (object, string key)
 *   - { tb: "user", id: { String: "abc" } }  (object, wrapped key)
 *
 * Returns the bare key ("abc"), or "" when no usable id is present.
 */
export function recordKey(value: unknown): string {
  if (!value) return '';
  if (typeof value === 'string') {
    // "user:abc" -> "abc"; bare "abc" stays "abc".
    const colon = value.indexOf(':');
    return colon >= 0 ? value.slice(colon + 1) : value;
  }
  if (typeof value !== 'object') return '';

  const inner = (value as Record<string, unknown>).id;
  if (typeof inner === 'string') return inner;
  if (inner && typeof inner === 'object') {
    const first = Object.values(inner as Record<string, unknown>)[0];
    return first != null ? String(first) : '';
  }
  return '';
}
