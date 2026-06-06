import { describe, expect, it } from 'vitest';
import {
  DEFAULT_LAYER_ID,
  normalizeLayerMetas,
  type LayerMeta,
} from './whiteboard-provider';

function layer(partial: Partial<LayerMeta>): LayerMeta {
  return {
    id: DEFAULT_LAYER_ID,
    name: 'Default',
    locked: false,
    z_index: 0,
    ...partial,
  };
}

describe('normalizeLayerMetas', () => {
  it('returns a synthetic default layer for a fresh whiteboard', () => {
    expect(normalizeLayerMetas([])).toEqual([
      { id: DEFAULT_LAYER_ID, name: 'Default', locked: false, z_index: 0 },
    ]);
  });

  it('deduplicates merged layer entries so keyed Svelte lists do not crash', () => {
    expect(
      normalizeLayerMetas([
        layer({ id: DEFAULT_LAYER_ID, name: 'Default', locked: false, z_index: 2 }),
        layer({ id: 'notes', name: 'Notes', locked: false, z_index: 1 }),
        layer({ id: DEFAULT_LAYER_ID, name: '', locked: true, z_index: 0 }),
      ]),
    ).toEqual([
      { id: DEFAULT_LAYER_ID, name: 'Default', locked: true, z_index: 0 },
      { id: 'notes', name: 'Notes', locked: false, z_index: 1 },
    ]);
  });

  it('ignores empty layer ids', () => {
    expect(
      normalizeLayerMetas([
        layer({ id: '', name: 'Broken', z_index: 0 }),
        layer({ id: 'valid', name: 'Valid', z_index: 1 }),
      ]),
    ).toEqual([{ id: 'valid', name: 'Valid', locked: false, z_index: 1 }]);
  });
});
