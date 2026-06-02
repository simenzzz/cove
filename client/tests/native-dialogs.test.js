// @ts-nocheck
import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const sourceRoot = join(process.cwd(), 'src');
const nativeDialogPattern = /\b(?:window\.|globalThis\.)?(?:alert|prompt|confirm)\s*\(/;

function sourceFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...sourceFiles(path));
    } else if (/\.(svelte|ts|js)$/.test(entry.name)) {
      files.push(path);
    }
  }
  return files;
}

describe('native browser dialogs', () => {
  it('are not used in frontend source', () => {
    const offenders = sourceFiles(sourceRoot).filter((file) =>
      nativeDialogPattern.test(readFileSync(file, 'utf8')),
    );

    expect(offenders).toEqual([]);
  });
});
