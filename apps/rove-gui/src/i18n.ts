import {createI18n} from 'vue-i18n';
import {ref,watch} from 'vue';
import en from './locales/en.ts';
import zh from './locales/zh-CN.ts';

export type UiLocale='zh-CN'|'en';
export type LocalePreference=UiLocale|'system';
const storage_key='rove-gui-locale';
export function resolve_locale(preference:string|null,languages:readonly string[]):UiLocale {
  if(preference==='zh-CN'||preference==='en')return preference;
  for(const language of languages){if(/^zh(?:-|$)/i.test(language))return 'zh-CN';if(/^en(?:-|$)/i.test(language))return 'en';}
  return 'en';
}
function saved_preference():LocalePreference {
  try{const value=globalThis.localStorage?.getItem(storage_key);if(value==='zh-CN'||value==='en')return value;}catch{/* Storage may be unavailable in a private WebView. */}
  return 'system';
}
function system_languages():readonly string[]{return typeof navigator==='undefined'?[]:navigator.languages?.length?navigator.languages:[navigator.language];}
export const locale_preference=ref<LocalePreference>(saved_preference());
export const i18n=createI18n({legacy:false,locale:resolve_locale(locale_preference.value,system_languages()),fallbackLocale:'en',messages:{en,'zh-CN':zh},missingWarn:false,fallbackWarn:false});
export const ui_locale=i18n.global.locale;
export function t(key:string,parameters:Record<string,string|number>={}):string{return i18n.global.t(key,parameters);}
export function set_locale(value:string):void {
  if(value!=='en'&&value!=='zh-CN'&&value!=='system')return;
  locale_preference.value=value;ui_locale.value=resolve_locale(value,system_languages());
  try{globalThis.localStorage?.setItem(storage_key,value);}catch{/* Switching still works for this session. */}
}
export function install_locale_effects():()=>void {
  const stop=watch(ui_locale,value=>{document.documentElement.lang=value;document.title=`Rove · ${t('ui.roamer')}`;},{immediate:true});
  const system_changed=()=>{if(locale_preference.value==='system')ui_locale.value=resolve_locale('system',system_languages());};
  const storage_changed=(event:StorageEvent)=>{if(event.key===storage_key){locale_preference.value=saved_preference();ui_locale.value=resolve_locale(locale_preference.value,system_languages());}};
  window.addEventListener('languagechange',system_changed);window.addEventListener('storage',storage_changed);
  return ()=>{stop();window.removeEventListener('languagechange',system_changed);window.removeEventListener('storage',storage_changed);};
}
export function format_datetime(value:string|null|undefined):string {
  if(!value)return '—';const date=new Date(value);if(!Number.isFinite(date.getTime()))return value;
  return new Intl.DateTimeFormat(ui_locale.value,{dateStyle:'medium',timeStyle:'short'}).format(date);
}
export function format_number(value:number):string{return new Intl.NumberFormat(ui_locale.value).format(value);}
export function state_label(value:string):string {const key=`state.${value}`;return i18n.global.te(key)?t(key):value;}
export function provider_label(value:string):string {const key=`providers.${value}`;return i18n.global.te(key)?t(key):value;}
// Translate only controlled interface notices/errors. Never apply this to user messages.
const known_notices=new Map<string,string>();
for(const catalog of [en,zh])for(const [key,value]of Object.entries(catalog.ui)){if(!value.includes('{'))known_notices.set(value,`ui.${key}`);}
export function notice_text(value:string):string {
  if(value.includes('Local agent response timed out'))return t('chat.transport_timeout');
  const key=known_notices.get(value);if(key)return t(key);
  const code=/^(?:Error: )?([a-z][a-z0-9_]+):/.exec(value)?.[1];
  return code&&i18n.global.te(`errors.${code}`)?`${code}: ${t(`errors.${code}`)}`:value;
}
