export const INTRO_KEY = 'rove-prototype-introduction-v1';
export interface Introduction { version: 1; completed: boolean; step: number; resetPending: boolean }
type Preferences = Pick<Storage, 'getItem' | 'setItem'>;
export function loadIntroduction(storage?: Preferences): Introduction {
  const fresh: Introduction = { version: 1, completed: false, step: 0, resetPending: false };
  try {
    const saved = JSON.parse(storage?.getItem(INTRO_KEY) ?? 'null');
    if (saved?.version !== 1) return fresh;
    if (saved.resetPending === true) { saveIntroduction(storage, fresh); return fresh; }
    return { ...fresh, completed: saved.completed === true, step: Number.isInteger(saved.step) ? Math.max(0, Math.min(3, saved.step)) : 0 };
  } catch { return fresh; }
}
export function saveIntroduction(storage: Preferences | undefined, state: Introduction): boolean {
  try { if (!storage) return false; storage.setItem(INTRO_KEY, JSON.stringify(state)); return true; } catch { return false; }
}
