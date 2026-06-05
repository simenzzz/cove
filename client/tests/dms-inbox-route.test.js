// @ts-nocheck
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const routePath = join(process.cwd(), 'src/routes/(app)/dms/+page.svelte');
const source = readFileSync(routePath, 'utf8');

describe('DM inbox route', () => {
  it('renders the reusable direct message list without opening a channel', () => {
    expect(source).toContain("from '$components/DirectMessageList.svelte'");
    expect(source).toContain('<DirectMessageList />');
    expect(source).not.toContain('ChatInput');
  });

  it('loads friends and DMs for the inbox target list', () => {
    expect(source).toContain('fetchFriends()');
    expect(source).toContain('fetchDms()');
    expect(source).toContain('dmTargets');
  });
});
