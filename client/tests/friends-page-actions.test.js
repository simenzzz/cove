// @ts-nocheck
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const pagePath = join(process.cwd(), 'src/routes/(app)/friends/+page.svelte');
const source = readFileSync(pagePath, 'utf8');

function sectionSource(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start);
  expect(start).toBeGreaterThanOrEqual(0);
  expect(end).toBeGreaterThan(start);
  return source.slice(start, end);
}

describe('friends page action placement', () => {
  it('keeps direct messaging out of pending requests', () => {
    const pending = sectionSource('<!-- Pending requests -->', '<!-- All friends -->');

    expect(pending).not.toContain('messageFriend');
    expect(pending).not.toContain('MessageCircle');
    expect(pending).not.toContain('> Message');
  });

  it('shows message beside remove only for accepted friends', () => {
    const accepted = sectionSource('<!-- All friends -->', '<div class="mt-9">');
    const messageIndex = accepted.indexOf('messageFriend(user)');
    const removeIndex = accepted.indexOf('remove(user)');

    expect(messageIndex).toBeGreaterThanOrEqual(0);
    expect(removeIndex).toBeGreaterThan(messageIndex);
  });

  it('guards messageFriend with accepted-friend state before opening a DM', () => {
    const handler = sectionSource('async function messageFriend', 'function isOnline');

    expect(handler).toContain('isAcceptedFriend(user, $friends)');
    expect(handler).toContain('You can only message accepted friends.');
    expect(handler.indexOf('isAcceptedFriend(user, $friends)')).toBeLessThan(handler.indexOf('openDm('));
  });
});
