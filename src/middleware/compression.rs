use async_compression::tokio::bufread::{BrotliEncoder, GzipEncoder, ZstdEncoder};
use bytes::Bytes;
use std::io::Cursor;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgo {
    Gzip,
    Brotli,
    Zstd,
    None,
}

impl CompressionAlgo {
    /// Returns the compression efficiency priority (higher = better compression)
    fn priority(enc: &str) -> u8 {
        match enc {
            "br" => 3,      // Brotli - best compression ratio
            "zstd" => 2,    // Zstd - excellent speed/ratio balance
            "gzip" => 1,    // Gzip - widely compatible
            "deflate" => 1, // Deflate - similar to gzip
            _ => 0,
        }
    }

    /// Parses Accept-Encoding header and selects the best compression algorithm.
    /// When multiple encodings have the same quality value, prefers the more efficient one
    /// (brotli > zstd > gzip).
    pub fn from_accept_encoding(header: &str) -> Self {
        let encodings: Vec<(&str, f32)> = header
            .split(',')
            .filter_map(|s| {
                let parts: Vec<&str> = s.trim().split(';').collect();
                let encoding = parts.first()?.trim();
                let quality = parts
                    .get(1)
                    .and_then(|q| q.trim().strip_prefix("q="))
                    .and_then(|q| q.parse().ok())
                    .unwrap_or(1.0);
                Some((encoding, quality))
            })
            .collect();

        let mut best: (&str, f32) = ("", -1.0);
        for (enc, q) in encodings {
            // Only consider supported encodings
            let priority = Self::priority(enc);
            if priority == 0 {
                continue;
            }

            // Select if: higher quality, OR same quality but better compression algorithm
            if q > best.1 || (q == best.1 && priority > Self::priority(best.0)) {
                best = (enc, q);
            }
        }

        match best.0 {
            "br" => CompressionAlgo::Brotli,
            "zstd" => CompressionAlgo::Zstd,
            "gzip" | "deflate" => CompressionAlgo::Gzip,
            _ => CompressionAlgo::None,
        }
    }

    pub fn content_encoding(&self) -> Option<&'static str> {
        match self {
            CompressionAlgo::Gzip => Some("gzip"),
            CompressionAlgo::Brotli => Some("br"),
            CompressionAlgo::Zstd => Some("zstd"),
            CompressionAlgo::None => None,
        }
    }
}

pub struct CompressionMiddleware {
    min_size: usize,
    _level: u32, // async-compression uses default levels; kept for future use
}

impl CompressionMiddleware {
    pub fn new(min_size: usize, level: u32) -> Self {
        Self {
            min_size,
            _level: level,
        }
    }

    pub async fn compress(&self, data: Bytes, algo: CompressionAlgo) -> anyhow::Result<Bytes> {
        if data.len() < self.min_size || algo == CompressionAlgo::None {
            return Ok(data);
        }

        let cursor = Cursor::new(data.as_ref());
        let reader = tokio::io::BufReader::new(cursor);

        let mut output = Vec::new();

        match algo {
            CompressionAlgo::Gzip => {
                let mut encoder = GzipEncoder::new(reader);
                encoder.read_to_end(&mut output).await?;
            }
            CompressionAlgo::Brotli => {
                let mut encoder = BrotliEncoder::new(reader);
                encoder.read_to_end(&mut output).await?;
            }
            CompressionAlgo::Zstd => {
                let mut encoder = ZstdEncoder::new(reader);
                encoder.read_to_end(&mut output).await?;
            }
            CompressionAlgo::None => return Ok(data),
        }

        Ok(Bytes::from(output))
    }

    pub fn should_compress(content_type: Option<&str>) -> bool {
        match content_type {
            Some(ct) => {
                ct.starts_with("text/")
                    || ct.contains("json")
                    || ct.contains("xml")
                    || ct.contains("javascript")
                    || ct.contains("css")
                    || ct.contains("html")
            }
            None => false,
        }
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new(1024, 4)
    }
}

impl Clone for CompressionMiddleware {
    fn clone(&self) -> Self {
        Self {
            min_size: self.min_size,
            _level: self._level,
        }
    }
}
