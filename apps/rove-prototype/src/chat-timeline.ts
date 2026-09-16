import type { Message, Run } from './domain.ts';
export type ChatEntry = { kind: 'message'; id: string; message: Message } | { kind: 'run'; id: string; run: Run; reply?: string };
// A job occupies one assistant bubble after its prompt; its final reply replaces its status text.
// Historical unlinked messages stay intact, and unlinked jobs remain visible at the end.
export function chatTimeline(messages: Message[], runs: Run[]): ChatEntry[] {
  const byId = new Map(runs.map(run => [run.id, run]));
  const replies = new Map(messages.filter(m => m.role === 'assistant' && m.run_id).map(m => [m.run_id!, m.text]));
  const emitted = new Set<string>(), entries: ChatEntry[] = [];
  const appendRun = (run: Run) => { if (!emitted.has(run.id)) { emitted.add(run.id); entries.push({ kind: 'run', id: `run-${run.id}`, run, reply: replies.get(run.id) }); } };
  for (const message of messages) {
    const run = message.run_id ? byId.get(message.run_id) : undefined;
    if (message.role === 'assistant' && run) continue;
    entries.push({ kind: 'message', id: message.id, message });
    if (message.role === 'user' && run) appendRun(run);
  }
  runs.forEach(appendRun);
  return entries;
}
