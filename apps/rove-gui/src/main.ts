import { createApp } from 'vue';
import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import 'primeicons/primeicons.css';
import App from './App.vue';
import './style.css';
import {i18n,install_locale_effects} from './i18n';
createApp(App).use(i18n).use(PrimeVue, { theme: { preset: Aura, options: { darkModeSelector: '.dark' } } }).mount('#app');
install_locale_effects();
