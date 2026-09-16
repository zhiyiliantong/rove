use serde_json::Value;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Retains partial reads when a caller cancels `next` (for example in select!).
/// After an invalid frame or I/O error, discard the reader and its connection.
#[derive(Default)]
pub struct FrameReader {
    prefix: [u8; 4],
    prefix_read: usize,
    data: Vec<u8>,
    data_read: usize,
    failed: bool,
}
impl FrameReader {
    pub async fn next(
        &mut self,
        reader: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<Value>> {
        if self.failed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Frame reader is closed",
            ));
        }
        let result = self.read_next(reader).await;
        self.failed = result.is_err();
        result
    }

    async fn read_next(
        &mut self,
        reader: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<Value>> {
        while self.prefix_read < 4 {
            let count = reader.read(&mut self.prefix[self.prefix_read..]).await?;
            if count == 0 {
                return if self.prefix_read == 0 {
                    Ok(None)
                } else {
                    Err(io::ErrorKind::UnexpectedEof.into())
                };
            }
            self.prefix_read += count;
        }
        let size = u32::from_be_bytes(self.prefix) as usize;
        if size == 0 || size > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid frame length",
            ));
        }
        if self.data.is_empty() {
            self.data.resize(size, 0);
        }
        while self.data_read < size {
            let count = reader.read(&mut self.data[self.data_read..]).await?;
            if count == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            self.data_read += count;
        }
        let result = serde_json::from_slice(&self.data)
            .map(Some)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid JSON frame"));
        self.prefix_read = 0;
        self.data_read = 0;
        self.data = Vec::new();
        result
    }
}

/// One-shot reader. Use `FrameReader` if cancelling and resuming a partial read.
pub async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<Option<Value>> {
    FrameReader::default().next(reader).await
}
pub async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), value: &Value) -> io::Result<()> {
    let data = serde_json::to_vec(value)?;
    if data.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Frame too large",
        ));
    }
    writer.write_all(&(data.len() as u32).to_be_bytes()).await?;
    writer.write_all(&data).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[tokio::test]
    async fn fragmented_and_coalesced_frames() {
        let (mut tx, mut rx) = tokio::io::duplex(16);
        let writer = tokio::spawn(async move {
            for _ in 0..2 {
                for byte in [0, 0, 0, 2, b'{', b'}'] {
                    tx.write_all(&[byte]).await.unwrap();
                }
            }
        });
        assert_eq!(read_frame(&mut rx).await.unwrap(), Some(json!({})));
        assert_eq!(read_frame(&mut rx).await.unwrap(), Some(json!({})));
        assert!(read_frame(&mut rx).await.unwrap().is_none());
        writer.await.unwrap();
    }
    #[tokio::test]
    async fn invalid_frames() {
        for data in [
            vec![0, 0, 0, 0],
            vec![0, 128, 0, 1],
            vec![0, 0],
            vec![0, 0, 0, 1, 255],
        ] {
            assert!(read_frame(&mut data.as_slice()).await.is_err());
        }
    }

    #[tokio::test]
    async fn cancelled_reads_keep_partial_prefix_and_body() {
        let mut wire = Vec::new();
        let expected = json!({"text":"漫游"});
        write_frame(&mut wire, &expected).await.unwrap();
        for cut in 1..wire.len() {
            let (mut tx, mut rx) = tokio::io::duplex(256);
            let mut reader = FrameReader::default();
            tx.write_all(&wire[..cut]).await.unwrap();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(2), reader.next(&mut rx))
                    .await
                    .is_err()
            );
            tx.write_all(&wire[cut..]).await.unwrap();
            write_frame(&mut tx, &json!({"next":true})).await.unwrap();
            drop(tx);
            assert_eq!(reader.next(&mut rx).await.unwrap(), Some(expected.clone()));
            assert_eq!(
                reader.next(&mut rx).await.unwrap(),
                Some(json!({"next":true}))
            );
            assert!(reader.next(&mut rx).await.unwrap().is_none());
        }
    }
}
