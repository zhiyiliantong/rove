import type {Json} from './api';
export const CONFIG_FILE_LIMIT = 128 * 1024;
function bytes_to_base64url(bytes:Uint8Array):string{
  let binary='';
  for(let offset=0;offset<bytes.length;offset+=8192)binary+=String.fromCharCode(...bytes.subarray(offset,offset+8192));
  return btoa(binary).replaceAll('+','-').replaceAll('/','_').replace(/=+$/,'');
}
function base64url_to_bytes(value:string):Uint8Array{
  if(!value||!/^[-_A-Za-z0-9]+$/.test(value)||value.length%4===1)throw new Error('invalid_base64_config');
  const standard=value.replaceAll('-','+').replaceAll('_','/').padEnd(Math.ceil(value.length/4)*4,'=');
  let binary:string;try{binary=atob(standard);}catch{throw new Error('invalid_base64_config');}
  return Uint8Array.from(binary,char=>char.charCodeAt(0));
}
export function encode_network_config(config:Json):string{
  if(!config||Array.isArray(config)||typeof config!=='object')throw new Error('invalid_config');
  const bytes=new TextEncoder().encode(JSON.stringify(config));
  if(bytes.length>CONFIG_FILE_LIMIT)throw new Error('file_too_large');
  return bytes_to_base64url(bytes);
}
export function decode_network_config(text:string):Json{
  const value=text.trim().replace(/^\uFEFF/,'').trim();
  if(new TextEncoder().encode(value).length>CONFIG_FILE_LIMIT*2)throw new Error('file_too_large');
  let json:string;try{json=new TextDecoder('utf-8',{fatal:true}).decode(base64url_to_bytes(value));}catch{throw new Error('invalid_base64_config');}
  let config:Json;try{config=JSON.parse(json) as Json;}catch{throw new Error('invalid_base64_config');}
  if(!config||Array.isArray(config)||typeof config!=='object')throw new Error('invalid_base64_config');
  return config;
}
/** File contents only populate the existing form; importing is a separate act. */
export function parse_config_file(text:string):{config:Json}{
  return {config:decode_network_config(text)};
}
