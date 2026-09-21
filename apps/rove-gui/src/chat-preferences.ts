import {ref,watch} from 'vue';
const key='rove-gui-chat-suggestions';
function initial(){try{return localStorage.getItem(key)!=='false';}catch{return true;}}
export const show_suggestions=ref(initial());
export const preference_error=ref(false);
watch(show_suggestions,value=>{try{localStorage.setItem(key,String(value));preference_error.value=false;}catch{preference_error.value=true;}},{flush:'sync'});
