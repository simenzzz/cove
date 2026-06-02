// @ts-nocheck
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const layoutPath = join(process.cwd(), 'src/routes/(app)/+layout.svelte');
const source = readFileSync(layoutPath, 'utf8');

describe('app top navigation', () => {
  it('places Messages between Friends and New post', () => {
    const friendsIndex = source.indexOf("label: 'Friends'");
    const messagesIndex = source.indexOf("label: 'Messages'");
    const newPostIndex = source.indexOf("label: 'New post'");

    expect(friendsIndex).toBeGreaterThanOrEqual(0);
    expect(messagesIndex).toBeGreaterThan(friendsIndex);
    expect(newPostIndex).toBeGreaterThan(messagesIndex);
  });

  it('routes Messages through the latest-DM redirect handler', () => {
    expect(source).toContain('openLatestDm');
    expect(source).toContain('MessageCircle');
    expect(source).not.toContain("href: '/dms'");
  });
});
