export const INTRO_KEY = 'rove-gui-introduction-v1';
export interface Introduction { version: 1; completed: boolean; step: number; reset_pending: boolean }
type Preferences = Pick<Storage, 'getItem' | 'setItem'>;
export function fresh_introduction(): Introduction { return {version:1,completed:false,step:0,reset_pending:false}; }
export function load_introduction(storage?: Preferences): Introduction {
  try {
    const saved = JSON.parse(storage?.getItem(INTRO_KEY) ?? 'null');
    if(saved?.version !== 1 || saved.reset_pending === true) return fresh_introduction();
    return {...fresh_introduction(), completed:saved.completed === true, step:Number.isInteger(saved.step)?Math.max(0,Math.min(3,saved.step)):0};
  } catch { return fresh_introduction(); }
}
export function save_introduction(storage: Preferences | undefined, state: Introduction): boolean {
  try { if(!storage)return false; const value=JSON.stringify(state);storage.setItem(INTRO_KEY,value);return storage.getItem(INTRO_KEY)===value; } catch { return false; }
}
