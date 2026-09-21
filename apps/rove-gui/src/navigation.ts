export type Page = 'sessions' | 'archives' | 'services' | 'networks' | 'settings' | 'models' | 'devices';
export function page_from_hash(hash: string): Page {
  const value = hash.replace(/^#\/?/, '').split('/')[0];
  return ['sessions', 'archives', 'services', 'networks', 'settings', 'models', 'devices'].includes(value) ? value as Page : 'sessions';
}
