use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use chrono::{Duration, Utc};
use rove_protocol::{ApiError, contract::BLOBS};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    path::Path as FsPath,
    sync::{Arc, Mutex},
};

pub struct BlobStore {
    db: Mutex<Connection>,
    public_url: url::Url,
    max_bytes: u64,
    max_blobs: u64,
}
impl BlobStore {
    pub fn open(
        path: &FsPath,
        public_url: url::Url,
        max_bytes: u64,
        max_blobs: u64,
    ) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            matches!(public_url.scheme(), "https" | "http")
                && public_url.host().is_some()
                && public_url.username().is_empty()
                && public_url.password().is_none()
                && public_url.query().is_none()
                && public_url.fragment().is_none(),
            "Invalid public URL"
        );
        anyhow::ensure!(
            max_bytes > 0 && max_blobs > 0,
            "Storage limits must be positive"
        );
        let db = Connection::open(path)?;
        db.pragma_update(None, "journal_mode", "WAL")?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS blobs (id TEXT PRIMARY KEY, envelope TEXT NOT NULL, created_at TEXT NOT NULL, expires_at TEXT NOT NULL, bytes INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS blobs_expiry ON blobs(expires_at);")?;
        Ok(Arc::new(Self {
            db: Mutex::new(db),
            public_url,
            max_bytes,
            max_blobs,
        }))
    }
    pub fn clean_expired(&self) -> anyhow::Result<usize> {
        Ok(self.db.lock().unwrap().execute(
            "DELETE FROM blobs WHERE expires_at <= ?1",
            [Utc::now().to_rfc3339()],
        )?)
    }
    fn upload(&self, value: Value) -> Result<Value, ApiError> {
        BLOBS.validate("BlobUpload", &value)?;
        let envelope = &value["envelope"];
        let invalid = || ApiError::invalid("Invalid envelope encoding or length");
        if B64
            .decode(envelope["nonce"].as_str().unwrap())
            .map_err(|_| invalid())?
            .len()
            != 12
        {
            return Err(invalid());
        }
        let len = B64
            .decode(envelope["ciphertext"].as_str().unwrap())
            .map_err(|_| invalid())?
            .len();
        if !(16..=128 * 1024).contains(&len) {
            return Err(invalid());
        }
        let data = envelope.to_string();
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction().map_err(db_error)?;
        let (count, bytes): (i64, i64) = tx
            .query_row(
                "SELECT count(*),COALESCE(sum(bytes),0) FROM blobs",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_error)?;
        if count as u64 >= self.max_blobs
            || (bytes as u64).saturating_add(data.len() as u64) > self.max_bytes
        {
            return Err(ApiError::new(
                429,
                "rate_limited",
                "Ciphertext storage quota reached; retry after expiration cleanup",
            ));
        }
        let mut random_id = [0u8; 16];
        getrandom::fill(&mut random_id).map_err(|_| unavailable())?;
        let id: String = random_id.iter().map(|b| format!("{b:02x}")).collect();
        let created = Utc::now();
        let expires = created + Duration::days(7);
        tx.execute(
            "INSERT INTO blobs(id,envelope,created_at,expires_at,bytes) VALUES(?1,?2,?3,?4,?5)",
            params![
                id,
                data,
                created.to_rfc3339(),
                expires.to_rfc3339(),
                data.len() as i64
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(
            json!({"blob_id":id,"created_at":created.to_rfc3339(),"expires_at":expires.to_rfc3339(),"download_url":self.public_url.join(&format!("/v1/blobs/{id}")).unwrap().as_str(),"share_base_url":self.public_url.join(&format!("/c/{id}")).unwrap().as_str()}),
        )
    }
    fn download(&self, id: &str) -> Result<Value, ApiError> {
        if id.len() != 32
            || !id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(ApiError::invalid("Invalid blob_id"));
        }
        let row: Option<(String, String, String)> = self
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT envelope,created_at,expires_at FROM blobs WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(db_error)?;
        let (envelope, created, expires) =
            row.ok_or_else(|| ApiError::new(404, "blob_not_found", "Share not found"))?;
        let expires_time =
            chrono::DateTime::parse_from_rfc3339(&expires).map_err(|_| unavailable())?;
        if expires_time <= Utc::now() {
            return Err(ApiError::new(
                410,
                "share_expired",
                "Share has expired; this does not revoke existing network membership",
            ));
        }
        Ok(
            json!({"blob_id":id,"envelope":serde_json::from_str::<Value>(&envelope).map_err(|_| unavailable())?,"created_at":created,"expires_at":expires}),
        )
    }
}
fn unavailable() -> ApiError {
    ApiError::new(503, "storage_unavailable", "Ciphertext storage unavailable")
}
fn db_error(_: rusqlite::Error) -> ApiError {
    unavailable()
}
fn reply(status: u16, result: Result<Value, ApiError>) -> Response {
    let (status, value) = match result {
        Ok(v) => (status, v),
        Err(e) => (e.status, e.body()),
    };
    (
        StatusCode::from_u16(status).unwrap(),
        [(header::CACHE_CONTROL, "no-store")],
        Json(value),
    )
        .into_response()
}
async fn upload(
    State(store): State<Arc<BlobStore>>,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let value = match body {
        Ok(body) => {
            serde_json::from_slice(&body).map_err(|_| ApiError::invalid("Invalid JSON body"))
        }
        Err(_) => Err(ApiError::new(
            413,
            "payload_too_large",
            "Request body exceeds 262144 bytes",
        )),
    };
    reply(201, value.and_then(|v| store.upload(v)))
}
async fn download(State(store): State<Arc<BlobStore>>, Path(id): Path<String>) -> Response {
    reply(200, store.download(&id))
}
pub fn router(store: Arc<BlobStore>) -> Router {
    Router::new()
        .route("/v1/blobs", post(upload))
        .route("/v1/blobs/{blob_id}", get(download))
        .route("/c/{blob_id}", get(download))
        .fallback(|| async {
            reply(
                404,
                Err(ApiError::new(404, "blob_not_found", "Resource not found")),
            )
        })
        .layer(DefaultBodyLimit::max(262144))
        .with_state(store)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reusable_expiry_and_storage_limits() {
        let store = BlobStore::open(
            FsPath::new(":memory:"),
            "https://config.example.com".parse().unwrap(),
            1024 * 1024,
            1,
        )
        .unwrap();
        let root: Value = serde_json::from_str(rove_protocol::BLOB_OPENAPI).unwrap();
        let upload = &root["components"]["schemas"]["BlobUpload"]["examples"][0];
        let created = store.upload(upload.clone()).unwrap();
        BLOBS.validate("BlobCreated", &created).unwrap();
        let id = created["blob_id"].as_str().unwrap();
        assert_eq!(store.download(id).unwrap(), store.download(id).unwrap());
        BLOBS
            .validate("Blob", &store.download(id).unwrap())
            .unwrap();
        assert_eq!(store.upload(upload.clone()).unwrap_err().status, 429);
        let mut with_key = upload.clone();
        with_key["key"] = json!("must-never-be-stored");
        assert_eq!(store.upload(with_key).unwrap_err().status, 400);
        store
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE blobs SET expires_at='2000-01-01T00:00:00+00:00'",
                [],
            )
            .unwrap();
        assert_eq!(store.download(id).unwrap_err().status, 410);
        assert_eq!(store.clean_expired().unwrap(), 1);
        assert_eq!(store.download(id).unwrap_err().status, 404);
    }
}
