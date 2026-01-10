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

        let mut best = ("", 0.0f32);
        for (enc, q) in encodings {
            if q > best.1 {
                match enc {
                    "br" | "zstd" | "gzip" | "deflate" => best = (enc, q),
                    _ => {}
                }
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
        Self { min_size, _level: level }
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
