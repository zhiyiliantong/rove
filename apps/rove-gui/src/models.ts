export interface ModelEntry { model_id:string; connection_id:string; name:string; model:string; tested_at?:string|null }
export interface ModelConnection { connection_id:string; provider:string; base_url:string; api_key_configured:boolean }
export interface ModelCatalog { connections:ModelConnection[]; models:ModelEntry[]; default_model_id:string|null; onboarding_session_id?:string|null }
export function generated_names(provider:string, models:string[], existing:string[]):string[] {
  const used=[...existing];
  const slug=(value:string)=>value.toLowerCase().split(/[^a-z0-9.-]+/).filter(Boolean).join('-');
  return models.map(model=>{
    const p=slug(provider),m=slug(model),stem=(m.startsWith(`${p}-`)?m:`${p}-${m}`).slice(0,230),prefix=`${stem}-`;
    const indices=used.filter(name=>name.startsWith(prefix)).map(name=>name.slice(prefix.length)).filter(s=>/^\d+$/.test(s)).map(Number);
    const name=`${prefix}${String(Math.max(0,...indices)+1).padStart(2,'0')}`;used.push(name);return name;
  });
}
