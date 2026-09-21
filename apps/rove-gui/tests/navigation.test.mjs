import test from 'node:test';
import assert from 'node:assert/strict';
import { page_from_hash } from '../src/navigation.ts';
test('hash navigation retains known secondary pages and falls back safely', () => {
  for (const page of ['sessions', 'services', 'networks', 'settings', 'models', 'devices']) assert.equal(page_from_hash('#/' + page), page);
  for (const hash of ['', '#/', '#/unknown', '#/javascript:alert(1)']) assert.equal(page_from_hash(hash), 'sessions');
});
