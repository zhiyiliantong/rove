// Deterministic prototype text only: no LLM request and no claim of completed device work.
export function replyPreview(prompt: string): string {
  if (/公式|富文本|markdown/i.test(prompt)) return [
    '### 富文本演示',
    '',
    '这是 **Markdown 与流式输出** 的界面示例，不是真实模型推理。',
    '',
    '- 说明段落可以单独复制',
    '- 代码与公式保留原始内容',
    '',
    '以圆的面积为例，半径为 $r$ 时：',
    '',
    '$$',
    'A = \\pi r^2',
    '$$',
    '',
    '```rust',
    'let area = std::f64::consts::PI * radius * radius;',
    '```',
    '',
    '> 提示：点击公式旁的复制按钮，可以取得 LaTeX 源码。',
  ].join('\n');
  return '**已收到请求。**\n\n我会先检查目标设备，再处理这项任务。你可以继续对话，或稍后回来查看结果。\n\n> 当前为演示流程，没有执行真实设备操作。';
}
