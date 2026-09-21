export const DRAFT_KEY='rove-gui-drafts-v1';
export function load_drafts(storage?:Pick<Storage,'getItem'>):Record<string,string>{
  try{
    const saved=JSON.parse(storage?.getItem(DRAFT_KEY)??'null');
    if(saved?.version!==1||!saved.drafts||typeof saved.drafts!=='object'||Array.isArray(saved.drafts))return {};
    return Object.fromEntries(Object.entries(saved.drafts).filter((entry):entry is [string,string]=>entry[0].length>0&&entry[0].length<=128&&typeof entry[1]==='string'&&entry[1].length<=65536));
  }catch{return {};}
}
export function save_drafts(storage:Pick<Storage,'setItem'>|undefined,drafts:Record<string,string>):boolean{
  try{if(!storage)return false;storage.setItem(DRAFT_KEY,JSON.stringify({version:1,drafts:Object.fromEntries(Object.entries(drafts).filter(([,value])=>value.length>0))}));return true;}catch{return false;}
}
