export type Page = 'sessions' | 'archives' | 'services' | 'networks' | 'settings' | 'models';
export function page_from_hash(hash: string): Page {
  const value = hash.replace(/^#\/?/, '').split('/')[0];
  if(value==='devices')return 'networks'; // Old bookmarks cannot restore a global execution target.
  return ['sessions', 'archives', 'services', 'networks', 'settings', 'models'].includes(value) ? value as Page : 'sessions';
}
