<script lang="ts">
  import { onDestroy } from 'svelte';
  import { toolState, type ToolKind } from '$lib/stores/whiteboards';

  const TOOLS: { kind: ToolKind; label: string }[] = [
    { kind: 'pen', label: 'Pen' },
    { kind: 'rect', label: 'Rect' },
    { kind: 'circle', label: 'Circle' },
    { kind: 'line', label: 'Line' },
    { kind: 'arrow', label: 'Arrow' },
    { kind: 'text', label: 'Text' },
    { kind: 'eraser', label: 'Eraser' },
    { kind: 'select', label: 'Select' },
  ];

  let current: ToolKind = $state('pen');
  let color = $state('#222222');
  let width = $state(3);

  const offToolState = toolState.subscribe((s) => {
    current = s.tool;
    color = s.color;
    width = s.strokeWidth;
  });

  onDestroy(offToolState);

  function setTool(kind: ToolKind) {
    toolState.update((s) => ({ ...s, tool: kind }));
  }
  function setColor(c: string) {
    toolState.update((s) => ({ ...s, color: c }));
  }
  function setWidth(w: number) {
    toolState.update((s) => ({ ...s, strokeWidth: w }));
  }
</script>

<div class="toolbar">
  {#each TOOLS as t}
    <button
      class:active={current === t.kind}
      onclick={() => setTool(t.kind)}
      type="button"
    >
      {t.label}
    </button>
  {/each}

  <label class="color">
    Color
    <input type="color" value={color} oninput={(e) => setColor(e.currentTarget.value)} />
  </label>

  <label class="width">
    Width
    <input
      type="range"
      min="1"
      max="20"
      value={width}
      oninput={(e) => setWidth(Number(e.currentTarget.value))}
    />
    <span>{width}px</span>
  </label>
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    padding: 0.6rem;
    background: var(--color-surface);
    border: 1px solid var(--color-line);
    border-radius: var(--radius-xl);
  }
  button {
    padding: 0.35rem 0.8rem;
    border: 1px solid var(--color-line-strong);
    background: var(--color-elevated);
    color: var(--color-linen-dim);
    cursor: pointer;
    border-radius: var(--radius-md);
    font-size: 0.85rem;
    transition:
      background 140ms ease,
      color 140ms ease,
      border-color 140ms ease;
  }
  button:hover {
    color: var(--color-linen);
    border-color: var(--color-copper);
  }
  button.active {
    background: var(--color-copper);
    color: var(--color-canvas);
    border-color: var(--color-copper);
  }
  .color,
  .width {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--color-linen-muted);
  }
  .color input {
    width: 32px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--color-line-strong);
    border-radius: var(--radius-sm);
    background: var(--color-elevated);
    cursor: pointer;
  }
  .width span {
    min-width: 2.5rem;
    text-align: right;
    color: var(--color-linen-dim);
  }
</style>
