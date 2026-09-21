import type {Json} from './api';

export function network_secret():string {
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), byte=>byte.toString(16).padStart(2,'0')).join('');
}

/** Normalize human CIDR input (including host bits); backend validates again. */
export function normalize_subnet(input:string):string {
  const [raw,prefix,...rest]=input.trim().split('/');
  const parts=raw?.split('.')??[];
  if(rest.length||parts.length!==4||!parts.every(p=>/^\d{1,3}$/.test(p)&&Number(p)<=255)||!/^\d{1,2}$/.test(prefix??''))throw new Error('invalid_subnet');
  const bits=Number(prefix);if(bits<1||bits>30)throw new Error('invalid_subnet');
  const address=parts.reduce((n,p)=>(n*256+Number(p))>>>0,0);
  const net=(address&(0xffffffff<<(32-bits)))>>>0;
  return `${[24,16,8,0].map(shift=>(net>>>shift)&255).join('.')}/${bits}`;
}

export function form_config(name:string,secret:string,peers:string[],automatic:boolean,cidr:string,original?:Record<string,Json>):Record<string,Json> {
  const config:Record<string,Json>={network_name:original?.network_name??name.trim(),network_secret:secret,bootstrap_peers:[...new Set(peers.map(v=>v.trim()).filter(Boolean))],dhcp:automatic};
  if(!automatic)config.ipv4_cidr=normalize_subnet(cidr);
  // Preserve historical DHCP metadata on edit, without treating it as a pool.
  else if(original?.dhcp===true&&original.ipv4_cidr)config.ipv4_cidr=original.ipv4_cidr;
  return config;
}
