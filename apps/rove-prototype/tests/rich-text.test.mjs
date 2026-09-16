import test from 'node:test';
import assert from 'node:assert/strict';
import { renderRichText } from '../src/rich-text.ts';
import { replyPreview } from '../src/reply-preview.ts';
import { createMockApi } from '../src/mock.ts';
test('Markdown has headings, explanation/code/formula copies and safely renders incomplete streams', () => {
  const source = replyPreview('演示公式与富文本'), result = renderRichText(source);
  assert.match(result.html, /<h3>/); assert.match(result.html, /<strong>/); assert.match(result.html, /<ul>/); assert.match(result.html, /class="katex/);
  assert.ok(result.copies.includes('A = \\pi r^2')); assert.ok(result.copies.some(s => s.includes('let area'))); assert.ok(result.copies.some(s => s.includes('Markdown')));
  for (let i = 1; i < source.length; i += 7) assert.doesNotThrow(() => renderRichText(source.slice(0, i)));
});
test('untrusted HTML, javascript links, remote images and unsafe formula commands do not execute or fetch', () => {
  const input = '<script>alert(1)</script>\n\n[x](javascript:alert(1))\n\n![private](https://example.invalid/track)\n\n$$\n\\includegraphics{https://example.invalid/image}\n$$\n\n```html\n<img onerror="alert(2)">\n```';
  const result = renderRichText(input);
  assert.doesNotMatch(result.html, /<script|<img|href="javascript:|onerror="/); assert.match(result.html, /&lt;script&gt;/); assert.match(result.html, /原型不自动加载/);
  assert.ok(result.copies.some(s => s.includes('<img onerror=')));
});
test('simulated reply prefixes grow, recover after reload, pause offline and stop on cancellation', () => {
  let t = 1000, saved; const storage = { getItem: () => saved ?? null, setItem: (_key, value) => { saved = value; } };
  const api = createMockApi(storage, () => t); const id = api.submit('welcome', '富文本公式', 'windows'); api.snapshot();
  t += 350; const a = api.snapshot().runs[0].output_text; t += 350; const b = api.snapshot().runs[0].output_text;
  assert.ok(b.length > a.length); assert.ok(b.startsWith(a));
  assert.equal(createMockApi(storage, () => t).snapshot().runs[0].output_text, b);
  api.disconnectNetwork('home'); t += 1000; assert.equal(api.snapshot().runs[0].output_text, b);
  api.cancelRun(id); t += 5000; assert.equal(api.snapshot().runs[0].output_text, b);
});
