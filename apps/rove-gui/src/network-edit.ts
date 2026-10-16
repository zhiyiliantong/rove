import type {Json} from './api';

export type EditStage='checking'|'stopping'|'saving'|'starting';
type Call=(operation:string,body?:Json,path?:Record<string,string>)=>Promise<Json>;
type State={state:string;last_error?:{message?:string}|null};
export class NetworkEditError extends Error {
  stage:EditStage; saved:boolean;
  constructor(stage:EditStage,saved:boolean,cause:unknown){super(cause instanceof Error?cause.message:String(cause));this.stage=stage;this.saved=saved;}
}

/** Shared by desktop and mobile; lifecycle acknowledgements are not completion. */
export async function save_network_edit(call:Call,id:string,config:Json,progress:(stage:EditStage)=>void=()=>{},sleep:(ms:number)=>Promise<void>=ms=>new Promise(resolve=>setTimeout(resolve,ms)),attempts=120){
  const path={network_id:id};let stage:EditStage='checking',saved=false;
  const phase=(value:EditStage)=>{stage=value;progress(value);};
  async function wait_for(expected:string){
    for(let i=0;i<attempts;i++){
      const network=await call('get_network',undefined,path) as unknown as State;
      if(network.state===expected)return;
      if(network.state==='failed')throw new Error(network.last_error?.message||'Network runtime failed');
      await sleep(500);
    }
    throw new Error(`Timed out waiting for network state: ${expected}`);
  }
  try{
    phase('checking');
    const original=await call('get_network',undefined,path) as unknown as State;
    const restart=original.state==='running'||original.state==='starting';
    if(original.state!=='stopped'){
      phase('stopping');await call('stop_network',undefined,path);await wait_for('stopped');
    }
    phase('saving');await call('update_network',config,path);saved=true;
    if(restart){phase('starting');await call('start_network',undefined,path);await wait_for('running');}
  }catch(cause){throw new NetworkEditError(stage,saved,cause);}
}
