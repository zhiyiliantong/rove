import test from 'node:test';
import assert from 'node:assert/strict';
import { demoId } from '../src/ids.ts';
test('demo UUID generation works without crypto.randomUUID on LAN HTTP', () => {
  const original = globalThis.crypto;
  Object.defineProperty(globalThis, 'crypto', {configurable: true, value: {getRandomValues: original.getRandomValues.bind(original)}});
  try {
    const ids = Array.from({length: 100}, demoId);
    assert.equal(new Set(ids).size, 100);
    for (const id of ids) assert.match(id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  } finally { Object.defineProperty(globalThis, 'crypto', {configurable: true, value: original}); }
});
