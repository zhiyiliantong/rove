import type {Json} from './api';
export const CONFIG_FILE_LIMIT = 128 * 1024;
/** File contents only populate the existing form; importing is a separate act. */
export function parse_config_file(text:string):{url:string}|{config:Json}{
  if(new TextEncoder().encode(text).length>CONFIG_FILE_LIMIT)throw new Error('file_too_large');
  const value=text.replace(/^\uFEFF/,'').trim();
  if(value.startsWith('{')){
    const config=JSON.parse(value) as Json;
    if(!config||Array.isArray(config)||typeof config!=='object')throw new Error('invalid_config_file');
    return {config};
  }
  const url=new URL(value);
  if(!['https:','http:'].includes(url.protocol)||!url.hash||url.username||url.password)throw new Error('invalid_config_file');
  return {url:value};
}
