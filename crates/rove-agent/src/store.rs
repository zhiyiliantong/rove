use fs2::FileExt;
use rove_core::DeviceId;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct Store {
    pub(crate) connection: Mutex<Connection>,
    pub device_id: DeviceId,
    pub data_dir: PathBuf,
    _lock: File,
}
impl Store {
    pub fn open(data_dir: &Path) -> anyhow::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, MetadataExt};
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(data_dir)?;
            let metadata = fs::symlink_metadata(data_dir)?;
            // SAFETY: geteuid has no preconditions and does not mutate memory.
            let uid = unsafe { libc::geteuid() };
            anyhow::ensure!(
                metadata.is_dir() && metadata.mode() & 0o077 == 0 && metadata.uid() == uid,
                "Data directory must be a private directory (mode 0700)"
            );
        }
        #[cfg(not(unix))]
        fs::create_dir_all(data_dir)?;
        let data_dir = fs::canonicalize(data_dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data_dir.join("agent.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|_| anyhow::anyhow!("An agent already owns this data directory"))?;
        let database = data_dir.join("rove.db");
        let db = Connection::open(&database)?;
        db.pragma_update(None, "foreign_keys", "ON")?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        anyhow::ensure!(
            version <= 5,
            "Database was created by a newer Rove; refusing downgrade"
        );
        if version == 0 {
            let tables: i64 = db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )?;
            if tables > 0 {
                let backup = data_dir.join(format!("rove-before-v1-{}.db", uuid::Uuid::new_v4()));
                db.backup("main", backup, None)?;
            }
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS networks (id TEXT PRIMARY KEY, record TEXT NOT NULL, join_config TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, record TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE, session_id TEXT NOT NULL REFERENCES sessions(id), record TEXT NOT NULL, request TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS messages (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id), run_id TEXT NOT NULL REFERENCES runs(id), record TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS run_events (run_id TEXT NOT NULL REFERENCES runs(id), seq INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY(run_id, seq));
                CREATE TABLE IF NOT EXISTS services (id TEXT PRIMARY KEY, network_id TEXT NOT NULL REFERENCES networks(id), record TEXT NOT NULL);
                PRAGMA user_version=1; COMMIT;")?;
        }
        if version < 2 {
            if version == 1 {
                db.backup(
                    "main",
                    data_dir.join(format!("rove-before-v2-{}.db", uuid::Uuid::new_v4())),
                    None,
                )?;
            }
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE IF NOT EXISTS rig_messages (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL REFERENCES sessions(id), run_id TEXT NOT NULL REFERENCES runs(id), message TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS run_output (run_id TEXT PRIMARY KEY REFERENCES runs(id), tail TEXT NOT NULL, truncated INTEGER NOT NULL);
                PRAGMA user_version=2; COMMIT;")?;
        }
        if version < 3 {
            if version > 0 {
                db.backup(
                    "main",
                    data_dir.join(format!("rove-before-v3-{}.db", uuid::Uuid::new_v4())),
                    None,
                )?;
            }
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE IF NOT EXISTS deleted_requests (request_id TEXT PRIMARY KEY);
                UPDATE sessions SET record=json_set(record,'$.archived_at',NULL) WHERE json_type(record,'$.archived_at') IS NULL;
                PRAGMA user_version=3; COMMIT;")?;
        }
        if version < 4 {
            if version > 0 {
                db.backup(
                    "main",
                    data_dir.join(format!("rove-before-v4-{}.db", uuid::Uuid::new_v4())),
                    None,
                )?;
            }
            db.execute_batch("BEGIN IMMEDIATE;")?;
            crate::models::migrate(&db)?;
            db.execute_batch("PRAGMA user_version=4; COMMIT;")?;
        }
        if version < 5 {
            if version > 0 {
                db.backup(
                    "main",
                    data_dir.join(format!("rove-before-v5-{}.db", uuid::Uuid::new_v4())),
                    None,
                )?;
            }
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE IF NOT EXISTS session_creations (id TEXT PRIMARY KEY, request TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS remote_submissions (request_id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), request TEXT NOT NULL, wire TEXT NOT NULL, run_id TEXT, created_at TEXT NOT NULL, error TEXT);
                CREATE TABLE IF NOT EXISTS remote_runs (id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), record TEXT NOT NULL, snapshot TEXT, observed_seq INTEGER NOT NULL DEFAULT 0);
                PRAGMA user_version=5; COMMIT;")?;
        }
        db.pragma_update(None, "journal_mode", "WAL")?;
        let id: Option<String> = db
            .query_row(
                "SELECT value FROM metadata WHERE key='device_id'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let device_id = if let Some(id) = id {
            id.parse()?
        } else {
            let id = DeviceId::new();
            db.execute(
                "INSERT INTO metadata(key,value) VALUES('device_id',?1)",
                [id.to_string()],
            )?;
            id
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&database, fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self {
            connection: Mutex::new(db),
            device_id,
            data_dir,
            _lock: lock,
        })
    }
    pub fn get(&self, key: &str) -> anyhow::Result<Option<Value>> {
        let db = self.connection.lock().unwrap();
        let value: Option<String> = db
            .query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        value
            .map(|v| serde_json::from_str(&v).map_err(Into::into))
            .transpose()
    }
    pub fn set(&self, key: &str, value: &Value) -> anyhow::Result<()> {
        self.connection.lock().unwrap().execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value.to_string()])?;
        Ok(())
    }
    pub fn settings(&self) -> anyhow::Result<Value> {
        Ok(self
            .get("settings")?
            .unwrap_or_else(|| json!({"max_active_runs":4,"config_server_url":null})))
    }
    pub fn model_state(&self) -> anyhow::Result<Value> {
        Ok(match self.get("model_config")? {
            Some(v) if !v.is_null() => {
                json!({"configured":true,"config":{"provider":v["provider"],"base_url":v["base_url"],"model":v["model"],"api_key_configured":v["api_key"].as_str().is_some_and(|s| !s.is_empty())}})
            }
            _ => json!({"configured":false,"config":null}),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_identity_single_writer_and_secret_redaction() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        let store = Store::open(&path).unwrap();
        let id = store.device_id;
        assert!(Store::open(&path).is_err());
        store.set("model_config", &json!({"provider":"openai_compatible","model":"demo","base_url":"http://localhost:11434/v1","api_key":"test-secret-never-print"})).unwrap();
        assert!(
            !store
                .model_state()
                .unwrap()
                .to_string()
                .contains("test-secret")
        );
        assert_eq!(store.settings().unwrap()["max_active_runs"], 4);
        drop(store);
        assert_eq!(Store::open(&path).unwrap().device_id, id);
    }

    #[test]
    fn v2_sessions_gain_archive_field_without_changing_identity_or_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        let original = Store::open(&path).unwrap();
        let id = original.device_id;
        let old = json!({"session_id":"legacy", "title":"保留历史"});
        {
            let db = original.connection.lock().unwrap();
            db.execute(
                "INSERT INTO sessions (id,record) VALUES (?1,?2)",
                rusqlite::params!["legacy", old.to_string()],
            )
            .unwrap();
            db.execute_batch("DROP TABLE deleted_requests; PRAGMA user_version=2;")
                .unwrap();
        }
        drop(original);
        let migrated = Store::open(&path).unwrap();
        assert_eq!(migrated.device_id, id);
        let record: String = migrated
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT record FROM sessions WHERE id='legacy'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let record: Value = serde_json::from_str(&record).unwrap();
        assert_eq!(record["title"], old["title"]);
        assert_eq!(record.get("archived_at"), Some(&Value::Null));
        let backup = std::fs::read_dir(&path)
            .unwrap()
            .map(|p| p.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("rove-before-v3-")
            })
            .unwrap();
        let db = Connection::open(backup).unwrap();
        let saved: String = db
            .query_row("SELECT record FROM sessions WHERE id='legacy'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&saved).unwrap(), old);
        assert_eq!(
            db.pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0))
                .unwrap(),
            2
        );
    }

    #[test]
    fn v4_upgrade_backs_up_and_preserves_identity_models_and_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        let original = Store::open(&path).unwrap();
        let id = original.device_id;
        let session =
            json!({"session_id":uuid::Uuid::new_v4(),"title":"existing history","device_id":id});
        let model = json!({"connections":[{"api_key":"private-upgrade-secret"}],"models":[]});
        original.set("model_catalog", &model).unwrap();
        {
            let db = original.connection.lock().unwrap();
            db.execute(
                "INSERT INTO sessions(id,record) VALUES(?1,?2)",
                params![session["session_id"].as_str(), session.to_string()],
            )
            .unwrap();
            db.execute_batch("DROP TABLE remote_runs; DROP TABLE remote_submissions; DROP TABLE session_creations; PRAGMA user_version=4;").unwrap();
        }
        drop(original);
        let migrated = Store::open(&path).unwrap();
        assert_eq!(migrated.device_id, id);
        assert_eq!(migrated.get("model_catalog").unwrap().unwrap(), model);
        let saved: String = migrated
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT record FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&saved).unwrap(), session);
        let backup = std::fs::read_dir(&path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("rove-before-v5-")
            })
            .unwrap();
        let before = Connection::open(backup).unwrap();
        assert_eq!(
            before
                .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
                .unwrap(),
            4
        );
        assert_eq!(
            migrated
                .connection
                .lock()
                .unwrap()
                .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
                .unwrap(),
            5
        );
    }

    #[test]
    fn migration_backs_up_existing_database_and_rejects_future_schema() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        let original = Store::open(&path).unwrap();
        let id = original.device_id;
        original
            .connection
            .lock()
            .unwrap()
            .pragma_update(None, "user_version", 0)
            .unwrap();
        drop(original);
        let migrated = Store::open(&path).unwrap();
        assert_eq!(migrated.device_id, id);
        let backups: Vec<_> = std::fs::read_dir(&path)
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("rove-before-v1-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).unwrap();
        let stored: String = backup
            .query_row(
                "SELECT value FROM metadata WHERE key='device_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, id.to_string());
        migrated
            .connection
            .lock()
            .unwrap()
            .pragma_update(None, "user_version", 6)
            .unwrap();
        drop(migrated);
        assert!(Store::open(&path).is_err());
    }
}
