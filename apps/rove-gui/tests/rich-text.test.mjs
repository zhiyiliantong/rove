import test from 'node:test';
import assert from 'node:assert/strict';
import { renderRichText } from '../src/rich-text.ts';
import {set_locale} from '../src/i18n.ts';
set_locale('zh-CN');
test('rich output supports incomplete streams and source copies', () => {
  const source = '### 标题\n\n**说明**\n\n- 项目\n\n```rust\nlet x = 1;\n```\n\n$$\nA = \\pi r^2\n$$';
  const result = renderRichText(source);
  assert.match(result.html, /<h3>/); assert.match(result.html, /<strong>/); assert.match(result.html, /class="katex/);
  assert.ok(result.copies.includes('A = \\pi r^2')); assert.ok(result.copies.includes('let x = 1;\n'));
  assert.ok(result.copies.includes('**说明**'));
  for (let i = 1; i < source.length; i++) assert.doesNotThrow(() => renderRichText(source.slice(0, i)));
});
test('untrusted output cannot inject HTML, unsafe URLs or remote images', () => {
  const result = renderRichText('<script>alert(1)</script>\n\n[x](javascript:alert(1))\n\n![private](https://example.invalid/track)\n\n$$\n\\includegraphics{https://example.invalid/image}\n$$\n\n```html\n<img onerror="alert(2)">\n```');
  assert.doesNotMatch(result.html, /<script|<img|href="javascript:|onerror="/);
  assert.match(result.html, /&lt;script&gt;/); assert.match(result.html, /不自动加载外部图片/);
  assert.ok(result.copies.some(s => s.includes('<img onerror=')));
});
