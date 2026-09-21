//! Bounded, metadata-only inspection. No arbitrary path API, recursion or cleanup.
use rove_protocol::ApiError;
use serde_json::{Value, json};
use std::{fs, path::Path};

const MAX_ENTRIES: usize = 512;

pub(crate) fn inspect(path: &Path, device: &str) -> Result<Value, ApiError> {
    let listing = fs::read_dir(path).map_err(|e| crate::storage_error(e.into()))?;
    let mut entries = Vec::new();
    let mut total = 0u64;
    let mut complete = true;
    for (index, entry) in listing.enumerate() {
        if index == MAX_ENTRIES {
            complete = false;
            break;
        }
        let Ok(entry) = entry else {
            complete = false;
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = fs::symlink_metadata(entry.path());
        let (kind, bytes) = match metadata {
            Ok(m) if m.file_type().is_symlink() => {
                complete = false;
                ("symlink", None)
            }
            Ok(m) if m.is_dir() => {
                complete = false;
                ("directory", None)
            }
            Ok(m) if m.is_file() => {
                let kind = match name.as_str() {
                    "rove.db" => "database",
                    "rove.db-wal" | "rove.db-shm" | "rove.db-journal" => "database_auxiliary",
                    "agent.lock" => "runtime",
                    _ if migration_backup(&name) => "migration_backup",
                    _ => "file",
                };
                if let Some(sum) = total.checked_add(m.len()) {
                    total = sum;
                } else {
                    complete = false;
                }
                (kind, Some(m.len()))
            }
            Ok(_) => ("runtime", None), // e.g. active Unix socket, never opened
            Err(_) => {
                complete = false;
                ("unreadable", None)
            }
        };
        entries.push(json!({"name":name,"kind":kind,"bytes":bytes}));
    }
    entries.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(
        json!({"device_id":device,"data_directory":path.to_string_lossy(),
        "scanned_at":rove_core::now(),"total_bytes":total,"complete":complete,"entries":entries}),
    )
}

fn migration_backup(name: &str) -> bool {
    name.strip_prefix("rove-before-v")
        .and_then(|s| s.strip_suffix(".db"))
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(version, id)| {
            version.parse::<u32>().is_ok() && id.parse::<uuid::Uuid>().is_ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Agent;
    use rove_protocol::{Request, contract::AGENT};

    #[tokio::test]
    async fn inspection_is_metadata_only_and_shared_with_management() {
        let tmp = tempfile::tempdir().unwrap();
        let agent = Agent::open(&tmp.path().join("agent")).unwrap();
        let backup = format!("rove-before-v5-{}.db", uuid::Uuid::new_v4());
        fs::write(
            agent.store.data_dir.join(&backup),
            "fixture-secret-never-read",
        )
        .unwrap();
        fs::create_dir(agent.store.data_dir.join("nested")).unwrap();
        fs::write(agent.store.data_dir.join("nested/large"), vec![0; 4096]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.path(), agent.store.data_dir.join("outside")).unwrap();
        let report = crate::management::call(&agent, &json!({"operation_id":"get_storage_usage"}))
            .await
            .unwrap()
            .unwrap();
        AGENT.validate("StorageUsage", &report).unwrap();
        assert_eq!(report["device_id"], agent.store.device_id.to_string());
        assert_eq!(report["complete"], false);
        assert!(!report.to_string().contains("fixture-secret"));
        let entries = report["entries"].as_array().unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e["name"] == backup && e["kind"] == "migration_backup")
        );
        assert!(
            entries
                .iter()
                .any(|e| e["name"] == "nested" && e["bytes"].is_null())
        );
        assert_eq!(
            report["total_bytes"].as_u64().unwrap(),
            entries
                .iter()
                .filter_map(|e| e["bytes"].as_u64())
                .sum::<u64>()
        );
        assert!(
            fs::read_to_string(agent.store.data_dir.join(&backup))
                .unwrap()
                .contains("fixture-secret")
        );
        let invalid = agent
            .handle(&Request::new("get_storage_usage").with_body(json!({"path":"/"})))
            .await;
        assert_eq!(invalid.status_code, 400);
        agent.shutdown().await;
    }

    #[test]
    fn inspection_is_bounded_and_does_not_claim_a_complete_total() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..MAX_ENTRIES + 1 {
            fs::write(tmp.path().join(i.to_string()), b"x").unwrap();
        }
        let report = inspect(tmp.path(), &uuid::Uuid::new_v4().to_string()).unwrap();
        assert_eq!(report["complete"], false);
        assert_eq!(report["entries"].as_array().unwrap().len(), MAX_ENTRIES);
        assert_eq!(report["total_bytes"], MAX_ENTRIES);
        assert!(!migration_backup("rove-before-v5-not-a-migration.db"));
    }
}
