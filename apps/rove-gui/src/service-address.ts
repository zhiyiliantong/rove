import {t} from './i18n.ts';
/** Only web service endpoints belong in a browser. Never pass app credentials,
 * local files, scripts, or arbitrary OS protocol handlers to the opener. */
export function browser_address(address:string):string{
  const url=new URL(address);
  if(!['http:','https:'].includes(url.protocol)||url.username||url.password||!url.hostname){
    throw new Error(t('ui.this_is_not_a_directly_accessible_http_https'));
  }
  return url.href;
}
export function can_open(address:string):boolean{
  try{browser_address(address);return true;}catch{return false;}
}
