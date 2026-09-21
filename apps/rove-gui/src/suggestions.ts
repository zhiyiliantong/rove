import {t} from './i18n.ts';
export function conversation_suggestions(){return [
 {icon:'pi-headphones',title:t('ui.bring_your_music_home'),text:t('ui.deploy_an_open_source_music_service_on_your'),prompt:t('ui.help_me_deploy_an_open_source_music_server')},
 {icon:'pi-th-large',title:t('ui.explore_my_services'),text:t('ui.find_apps_already_available_on_your_devices'),prompt:t('ui.list_the_services_published_by_this_device')},
 {icon:'pi-video',title:t('ui.build_your_private_cinema'),text:t('ui.host_open_source_video_services_and_watch_across'),prompt:t('ui.help_me_plan_an_open_source_media_server')},
 {icon:'pi-code',title:t('ui.manage_multiple_coding_agents'),text:t('ui.deploy_and_access_open_source_coding_agents_on'),prompt:t('ui.help_me_deploy_a_coding_agent_on_this')},
 {icon:'pi-compass',title:t('ui.manage_rove'),text:t('ui.inspect_storage_adjust_locations_and_troubleshoot'),prompt:t('ui.check_rove_s_storage_usage_and_runtime_on')}
];}
