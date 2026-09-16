// UI contract v4: intentionally separate from the shipping rove-agent API.
import type { CatalogRequest, ModelCatalog } from './model-catalog.ts';
import type { Provider } from './providers.ts';
export type Scenario = 'daily' | 'empty' | 'offline' | 'failure' | 'unsupported';
export type RunStatus = 'queued' | 'running' | 'cancelling' | 'succeeded' | 'failed' | 'cancelled' | 'interrupted';
export interface Connection { id: string; name: string; provider: string; base_url: string; auth_kind: 'api_key' | 'official_agent'; credential_ref: string }
export interface Model { id: string; connection_id: string; model: string; name: string }
export interface NetworkCard { name: string; subnet: string; network_key: string; initial_peers: string[] }
export interface Network extends NetworkCard { id: string; connection_status: 'disconnected' | 'waiting_dhcp' | 'connected' | 'conflict'; resolved_subnet: string | null; local_ip: string | null; connection_error: string | null; connect_started_at: number | null }
export interface PeerProbe { peer: string; status: 'reachable' | 'timed_out' | 'invalid'; latency_ms: number | null; simulated: true; message: string }
export interface ModelCopy { source_model_id: string; model: string; connection_name: string; base_url: string; credential_ref: string }
export interface Device { id: string; network_id: string; name: string; os: string; ip: string; online: boolean; model_ids: string[]; model_copies: ModelCopy[] }
export interface Service { id: string; device_id: string; network_id: string; name: string; address: string; reachable: boolean; kind: 'music' | 'files' | 'code' }
export interface Message { id: string; role: 'user' | 'assistant'; text: string; run_id?: string }
export interface Session { archived_at: number | null; id: string; title: string; owner_device_id: string; model_id: string; draft: string; target_device_id: string; onboarding: boolean; messages: Message[] }
export interface Run { output_text?: string; id: string; session_id: string; device_id: string; network_id: string | null; model_id: string; status: RunStatus; submitted_at: number; started_at: number | null; progress: number; outcome: 'success' | 'failure' | 'unsupported'; waiting_connection: boolean; intent: 'install_music' | 'rename_service' | 'remove_service' | 'query'; service_id: string | null; service_name: string | null; error: string | null }
export interface Snapshot { version: 4; scenario: Scenario; initialized: boolean; connections: Connection[]; models: Model[]; default_model_id: string | null; networks: Network[]; devices: Device[]; services: Service[]; sessions: Session[]; runs: Run[]; max_active_runs: number }
export interface ModelImport { name: string; provider: string; base_url: string; auth_kind: Connection['auth_kind']; models: string[]; model_names?: Record<string, string> }
export interface PrototypeApi {
  snapshot(): Snapshot;
  reset(scenario: Scenario): void;
  listProviders(): Provider[];
  importModels(input: ModelImport): string[];
  listModelCatalog(input: CatalogRequest): ModelCatalog;
  editConnection(id: string, patch: Pick<Connection, 'name' | 'base_url'>): void;
  deleteModel(id: string): void;
  setDefaultModel(id: string): void;
  createSession(): string;
  archiveSession(id: string): void;
  restoreSession(id: string): void;
  deleteSession(id: string): void;
  setSessionModel(id: string, modelId: string): void;
  saveDraft(id: string, draft: string): void;
  setSessionTarget(id: string, deviceId: string): void;
  submit(sessionId: string, text: string, deviceId: string): string;
  cancelRun(id: string): void;
  saveNetwork(card: NetworkCard, id?: string): string;
  connectNetwork(id: string): void;
  disconnectNetwork(id: string): void;
  probePeer(peer: string): Promise<PeerProbe>;
  deleteNetwork(id: string): void;
  syncModels(deviceId: string, modelIds: string[]): void;
  setConcurrency(value: number): void;
}
