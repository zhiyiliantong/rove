//! Pipe framing is shared; only the legacy Windows code-page conversion is
//! native. Never change the parent console or the user's system code page.
use std::io;

pub(super) struct Decoder {
    pending: Vec<u8>,
    legacy: Option<fn(&[u8]) -> io::Result<String>>,
}

impl Decoder {
    pub(super) fn native() -> Self {
        Self {
            pending: Vec::new(),
            #[cfg(windows)]
            legacy: Some(windows_output),
            #[cfg(not(windows))]
            legacy: None,
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8], eof: bool) -> io::Result<String> {
        self.pending.extend_from_slice(bytes);
        let mut output = String::new();
        loop {
            let end = if self.legacy.is_none() {
                // Preserve prompt streaming and incomplete UTF-8 across reads on
                // Unix. Invalid bytes retain the previous replacement semantics.
                match std::str::from_utf8(&self.pending) {
                    Err(e) if e.error_len().is_none() && !eof => e.valid_up_to(),
                    _ => self.pending.len(),
                }
            } else if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                end + 1
            } else if eof {
                self.pending.len()
            } else if self.pending.len() >= 4096 {
                // Bound non-line-oriented output without cutting UTF-8 or a DBCS
                // character. The Windows converter below supplies this boundary.
                legacy_boundary(&self.pending)
            } else {
                break;
            };
            if end == 0 {
                break;
            }
            let bytes = &self.pending[..end];
            let text = match std::str::from_utf8(bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => match self.legacy {
                    Some(decode) => decode(bytes)?,
                    None => String::from_utf8_lossy(bytes).into_owned(),
                },
            };
            self.pending.drain(..end);
            output.push_str(&text);
        }
        Ok(output)
    }
}

fn legacy_boundary(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Ok(_) => return bytes.len(),
        Err(e) if e.error_len().is_none() => return e.valid_up_to(),
        _ => {}
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::IsDBCSLeadByteEx;
        let codepage = windows_codepage();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let size = if unsafe { IsDBCSLeadByteEx(codepage, bytes[cursor]) } != 0 {
                2
            } else {
                1
            };
            if cursor + size > bytes.len() {
                break;
            }
            cursor += size;
        }
        cursor
    }
    #[cfg(not(windows))]
    bytes.len()
}

#[cfg(windows)]
fn windows_codepage() -> u32 {
    use windows_sys::Win32::{Globalization::GetOEMCP, System::Console::GetConsoleOutputCP};
    // GUI-started agents usually have no console; cmd/native console tools
    // then use the OEM code page. Attached CLI agents may inherit another one.
    let console = unsafe { GetConsoleOutputCP() };
    if console == 0 {
        unsafe { GetOEMCP() }
    } else {
        console
    }
}

#[cfg(windows)]
fn windows_output(bytes: &[u8]) -> io::Result<String> {
    decode_codepage(bytes, windows_codepage())
}

#[cfg(windows)]
fn decode_codepage(bytes: &[u8], codepage: u32) -> io::Result<String> {
    use windows_sys::Win32::Globalization::MultiByteToWideChar;
    if bytes.is_empty() {
        return Ok(String::new());
    }
    let mut wide = vec![0u16; bytes.len()];
    let count = unsafe {
        MultiByteToWideChar(
            codepage,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            wide.len() as i32,
        )
    };
    if count == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&wide[..count as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_utf8_and_eof() {
        let mut decoder = Decoder {
            pending: vec![],
            legacy: None,
        };
        let mut text = String::new();
        for byte in "回复：漫游者\n".as_bytes() {
            text += &decoder.push(&[*byte], false).unwrap();
        }
        text += &decoder.push(&[0xe4], true).unwrap();
        assert_eq!(text, "回复：漫游者\n�");
    }

    #[test]
    fn windows_framing_preserves_split_legacy_bytes_and_utf8() {
        fn legacy(bytes: &[u8]) -> io::Result<String> {
            assert_eq!(bytes, &[0xbb, 0xd8, 0xb8, 0xb4, b'\n']);
            Ok("回复\n".into())
        }
        let mut decoder = Decoder {
            pending: vec![],
            legacy: Some(legacy),
        };
        for byte in [0xbb, 0xd8, 0xb8, 0xb4] {
            assert_eq!(decoder.push(&[byte], false).unwrap(), "");
        }
        assert_eq!(decoder.push(b"\n", false).unwrap(), "回复\n");
        assert_eq!(
            decoder.push("UTF-8 中文\n".as_bytes(), false).unwrap(),
            "UTF-8 中文\n"
        );
        assert_eq!(decoder.push(b"no newline", true).unwrap(), "no newline");
        assert_eq!(decoder.push(&vec![b'x'; 8192], false).unwrap().len(), 8192);
        assert!(decoder.pending.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn actual_windows_cp936_and_stream_boundaries() {
        let legacy = |bytes: &[u8]| decode_codepage(bytes, 936);
        let mut decoder = Decoder {
            pending: vec![],
            legacy: Some(legacy),
        };
        let mut text = String::new();
        for byte in [0xbb, 0xd8, 0xb8, 0xb4, b'\r', b'\n'] {
            text += &decoder.push(&[byte], false).unwrap();
        }
        assert_eq!(text, "回复\r\n");
        assert_eq!(
            decode_codepage(&[0xb4, 0xed, 0xce, 0xf3], 936).unwrap(),
            "错误"
        );
    }
}
