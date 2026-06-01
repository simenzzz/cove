export interface WhiteboardPeerCursor {
  userId: string;
  label: string;
  x: number;
  y: number;
  color: string;
  tool: string;
}

interface CursorPoint {
  x: number;
  y: number;
}

interface WhiteboardAwarenessBlob {
  cursor?: CursorPoint;
  color?: string;
  tool?: string;
  display_name?: string;
  displayName?: string;
  username?: string;
}

function nonEmptyString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : null;
}

function isCursorPoint(value: unknown): value is CursorPoint {
  if (!value || typeof value !== 'object') return false;
  const point = value as Partial<CursorPoint>;
  return typeof point.x === 'number' && typeof point.y === 'number';
}

export function whiteboardCursorLabel(userId: string, blob: WhiteboardAwarenessBlob): string {
  return (
    nonEmptyString(blob.display_name) ??
    nonEmptyString(blob.displayName) ??
    nonEmptyString(blob.username) ??
    userId.slice(0, 8)
  );
}

export function whiteboardPeerCursors(
  peers: Record<string, unknown>,
): WhiteboardPeerCursor[] {
  return Object.entries(peers).flatMap(([userId, raw]) => {
    const blob = raw as WhiteboardAwarenessBlob;
    if (!isCursorPoint(blob?.cursor)) return [];

    return [
      {
        userId,
        label: whiteboardCursorLabel(userId, blob),
        x: blob.cursor.x,
        y: blob.cursor.y,
        color: blob.color ?? '#888',
        tool: blob.tool ?? 'pen',
      },
    ];
  });
}
