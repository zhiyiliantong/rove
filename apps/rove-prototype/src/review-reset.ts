import { seed, STORAGE_KEY } from './mock.ts';
import { INTRO_KEY, type Introduction } from './introduction.ts';
export function persistCleanReview(storage?: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>) {
  if (!storage) throw new Error('浏览器不允许本地存储，无法确认旧数据已清空。');
  const fresh: Introduction = {version:1,completed:false,step:0,resetPending:false};
  const writes = [[INTRO_KEY, JSON.stringify(fresh)], [STORAGE_KEY, JSON.stringify(seed('empty'))]];
  const previous = writes.map(([key]) => [key!, storage.getItem(key!)] as const);
  try {
    for (const [key, value] of writes) {
      storage.setItem(key!, value!);
      if (storage.getItem(key!) !== value) throw new Error('写入校验失败');
    }
  } catch {
    for (const [key, value] of previous) {
      try { if (value === null) storage.removeItem(key); else storage.setItem(key, value); } catch { /* Report failure, never claim success. */ }
    }
    throw new Error('未能完成清空：浏览器拒绝保存或写入校验失败。请关闭其他原型标签页并允许本地存储后重试。');
  }
}
