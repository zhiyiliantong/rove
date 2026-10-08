use qrcodegen::{QrCode, QrCodeEcc};
use std::fmt::Write;

/// Presentation of the existing share URL. Never uploads data or embeds
/// unescaped input into markup; pixels contain the fragment key too.
pub struct ShareQr(QrCode);
impl ShareQr {
    /// Width of the square module matrix, excluding the four-module quiet zone.
    pub fn size(&self) -> usize {
        self.0.size() as usize
    }
    /// Whether a module is dark. Coordinates outside the matrix are light.
    pub fn is_dark(&self, x: i32, y: i32) -> bool {
        self.0.get_module(x, y)
    }
    pub fn new(url: &str) -> anyhow::Result<Self> {
        anyhow::ensure!(url.len() <= 4096, "Share URL is too long for a QR code");
        let parsed = reqwest::Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid share URL"))?;
        anyhow::ensure!(
            matches!(parsed.scheme(), "http" | "https") && parsed.fragment().is_some(),
            "Expected a share URL including its fragment key"
        );
        Self::from_text(url)
    }
    /// Encode a self-contained card without applying legacy share-URL rules.
    pub fn from_text(text: &str) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !text.is_empty() && text.len() <= 4096,
            "QR payload is empty or too long"
        );
        QrCode::encode_text(text, QrCodeEcc::Medium)
            .map(Self)
            .map_err(|_| {
                anyhow::anyhow!(
                    "Configuration does not fit a QR code; copy the configuration instead"
                )
            })
    }
    pub fn svg(&self) -> String {
        let size = self.0.size() + 8;
        let mut path = String::new();
        for y in 0..self.0.size() {
            for x in 0..self.0.size() {
                if self.0.get_module(x, y) {
                    write!(path, "M{},{}h1v1h-1z", x + 4, y + 4).unwrap();
                }
            }
        }
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="white"/><path d="{path}" fill="black"/></svg>"#
        )
    }
    pub fn terminal(&self) -> String {
        let mut output = String::new();
        for y in (-4..self.0.size() + 4).step_by(2) {
            for x in -4..self.0.size() + 4 {
                // Light modules are filled to preserve a white quiet zone on
                // common dark terminals; no terminal escape codes are emitted.
                output.push(
                    match (self.0.get_module(x, y), self.0.get_module(x, y + 1)) {
                        (false, false) => '█',
                        (false, true) => '▀',
                        (true, false) => '▄',
                        (true, true) => ' ',
                    },
                );
            }
            output.push('\n');
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base64_card_qr_decodes_to_exact_configuration() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../apps/rove-gui/tests/browser/network-qr.json"
        ))
        .unwrap();
        let payload = fixture["payload"].as_str().unwrap();
        // The old URL-only path rejects this exact GUI payload.
        assert!(ShareQr::new(payload).is_err());
        let qr = ShareQr::from_text(payload).unwrap();
        let scale = 6;
        let side = (qr.size() + 8) * scale;
        let mut pixels = vec![255u8; side * side];
        for y in 0..side {
            for x in 0..side {
                if qr.is_dark((x / scale) as i32 - 4, (y / scale) as i32 - 4) {
                    pixels[y * side + x] = 0;
                }
            }
        }
        let mut decoder = quircs::Quirc::default();
        let codes: Vec<_> = decoder.identify(side, side, &pixels).collect();
        assert_eq!(codes.len(), 1);
        assert_eq!(
            codes[0].as_ref().unwrap().decode().unwrap().payload,
            payload.as_bytes()
        );
        assert!(ShareQr::from_text("").is_err());
        assert!(ShareQr::from_text(&"A".repeat(4097)).is_err());
    }
    #[test]
    fn qr_decodes_to_exact_url_including_fragment_key() {
        let url = "https://config.example.com/c/00112233445566778899aabbccddeeff#key=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let qr = ShareQr::new(url).unwrap();
        let scale = 6;
        let side = (qr.0.size() as usize + 8) * scale;
        let mut pixels = vec![255u8; side * side];
        for y in 0..side {
            for x in 0..side {
                if qr
                    .0
                    .get_module((x / scale) as i32 - 4, (y / scale) as i32 - 4)
                {
                    pixels[y * side + x] = 0;
                }
            }
        }
        let mut decoder = quircs::Quirc::default();
        let codes: Vec<_> = decoder.identify(side, side, &pixels).collect();
        assert_eq!(codes.len(), 1);
        let decoded = codes[0].as_ref().unwrap().decode().unwrap();
        assert_eq!(decoded.payload, url.as_bytes());
        let svg = qr.svg();
        assert!(!svg.contains(url) && !svg.contains("script"));
        assert!(svg.contains("crispEdges"));
        assert!(!qr.terminal().contains('\x1b'));
        assert!(ShareQr::new("https://example.com/no-key").is_err());
    }
}
