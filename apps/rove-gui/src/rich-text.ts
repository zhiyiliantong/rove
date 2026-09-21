import MarkdownIt from 'markdown-it';
import katex from 'katex';
import {t} from './i18n.ts';
export interface RichDocument extends Record<PropertyKey, unknown> { html: string; copies: string[] }
const md = new MarkdownIt({ html: false, breaks: true, linkify: false, maxNesting: 20 });
const escape = md.utils.escapeHtml;
function copyButton(env: unknown, value: string, label: string) {
  const index = (env as RichDocument).copies.push(value) - 1;
  return `<button type="button" class="segment-copy" data-copy-index="${index}" aria-label="${escape(t('rich.copyLabel',{label,index:index+1}))}" title="${escape(t('rich.copyTitle',{label}))}"><span class="pi pi-copy" aria-hidden="true"></span></button>`;
}
function math(source: string, displayMode: boolean) {
  return katex.renderToString(source, { displayMode, throwOnError: false, trust: false, strict: 'ignore', maxExpand: 100, maxSize: 20 });
}
md.inline.ruler.before('escape', 'inline_math', (state, silent) => {
  const bracket = state.src.startsWith('\\(', state.pos);
  if (!bracket && (state.src[state.pos] !== '$' || state.src[state.pos + 1] === '$')) return false;
  const open = bracket ? 2 : 1, close = bracket ? '\\)' : '$';
  const start = state.pos + open;
  if (/\s/.test(state.src[start] ?? ' ')) return false;
  let end = state.src.indexOf(close, start);
  while (end > 0 && state.src[end - 1] === '\\' && !bracket) end = state.src.indexOf(close, end + 1);
  if (end < 0 || end === start) return false;
  if (!silent) { const token = state.push('inline_math', '', 0); token.content = state.src.slice(start, end); }
  state.pos = end + close.length; return true;
});
md.block.ruler.before('fence', 'block_math', (state, startLine, endLine, silent) => {
  const first = state.src.slice(state.bMarks[startLine]! + state.tShift[startLine]!, state.eMarks[startLine]!).trim();
  if (!first.startsWith('$$')) return false;
  let source = first.slice(2), next = startLine + 1;
  while (!source.trimEnd().endsWith('$$') && next < endLine) { source += '\n' + state.src.slice(state.bMarks[next]!, state.eMarks[next]!); next++; }
  if (!source.trimEnd().endsWith('$$')) return false;
  if (silent) return true;
  const token = state.push('block_math', '', 0); token.content = source.trimEnd().slice(0, -2).trim(); token.block = true; token.map = [startLine, next]; state.line = next; return true;
});
md.renderer.rules.inline_math = (tokens, index, _options, env) => `<span class="inline-formula">${math(tokens[index]!.content, false)}${copyButton(env, tokens[index]!.content, t('rich.formula'))}</span>`;
md.renderer.rules.block_math = (tokens, index, _options, env) => `<div class="rich-segment formula-block">${copyButton(env, tokens[index]!.content, t('rich.formula'))}<div class="formula-scroll">${math(tokens[index]!.content, true)}</div></div>`;
md.renderer.rules.fence = (tokens, index, _options, env) => `<div class="rich-segment code-block">${copyButton(env, tokens[index]!.content, t('rich.code'))}<pre><code>${escape(tokens[index]!.content)}</code></pre></div>`;
md.renderer.rules.code_block = md.renderer.rules.fence;
md.renderer.rules.paragraph_open = (tokens, index, _options, env) => tokens[index]!.hidden ? '' : `<div class="rich-segment explanation-block">${copyButton(env, tokens[index + 1]?.content ?? '', t('rich.explanation'))}<p>`;
md.renderer.rules.paragraph_close = (tokens, index) => tokens[index]!.hidden ? '' : '</p></div>';
md.renderer.rules.image = (tokens, index) => `<span class="image-placeholder">${escape(t('rich.imagePlaceholder',{text:tokens[index]!.content}))}</span>`;
const originalLink = md.renderer.rules.link_open;
md.renderer.rules.link_open = (tokens, index, options, env, self) => {
  tokens[index]!.attrSet('target', '_blank'); tokens[index]!.attrSet('rel', 'noopener noreferrer');
  return originalLink ? originalLink(tokens, index, options, env, self) : self.renderToken(tokens, index, options);
};
export function renderRichText(source: string): RichDocument {
  const result: RichDocument = { html: '', copies: [] };
  result.html = md.render(source, result); return result;
}
