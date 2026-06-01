import { describe, it, expect } from 'vitest';
import { recordKey } from './record-id';

describe('recordKey', () => {
  it('returns "" for null / undefined / falsy', () => {
    expect(recordKey(null)).toBe('');
    expect(recordKey(undefined)).toBe('');
    expect(recordKey('')).toBe('');
  });

  it('strips the table prefix from a "table:id" string', () => {
    expect(recordKey('user:abc')).toBe('abc');
    expect(recordKey('channel:xyz123')).toBe('xyz123');
  });

  it('passes a bare id string through unchanged', () => {
    expect(recordKey('abc')).toBe('abc');
  });

  it('keeps only the first segment after the first colon', () => {
    // SurrealDB ids do not contain extra colons, but verify slicing semantics.
    expect(recordKey('user:a:b')).toBe('a:b');
  });

  it('reads the id from a { tb, id } object with a string key', () => {
    expect(recordKey({ tb: 'user', id: 'abc' })).toBe('abc');
  });

  it('unwraps a codec-wrapped key { tb, id: { String } }', () => {
    expect(recordKey({ tb: 'user', id: { String: 'abc' } })).toBe('abc');
  });

  it('unwraps the first value of any wrapper object shape', () => {
    expect(recordKey({ id: { Number: 42 } })).toBe('42');
  });

  it('returns "" for non-string / non-object primitives', () => {
    expect(recordKey(42)).toBe('');
    expect(recordKey(true)).toBe('');
  });

  it('returns "" when an object has no usable id field', () => {
    expect(recordKey({ tb: 'user' })).toBe('');
    expect(recordKey({})).toBe('');
  });
});
