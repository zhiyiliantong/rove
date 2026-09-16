import { invoke } from '@tauri-apps/api/core';
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export interface Target {network_id:string;device_id:string}
export async function call(operation_id: string, body?: Json, path_parameters?: Record<string,string>, query_parameters?: Record<string,string|number|boolean>, target?: Target): Promise<Json> {
  const response = await invoke<{status_code:number;body?:Json}>('agent_call', {
    request: {kind:'request',correlation_id:crypto.randomUUID(),operation_id,body,path_parameters,query_parameters,target},
  });
  if (response.status_code >= 400) {
    const error = response.body as {error:{code:string;message:string}};
    throw new Error(`${error.error.code}: ${error.error.message}`);
  }
  return response.body ?? null;
}
