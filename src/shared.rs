#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HttpVersion {
    OnePointOne,
}

impl From<&[u8]> for HttpVersion {
    fn from(value: &[u8]) -> Self {
        match value {
            b"HTTP/1.1" => HttpVersion::OnePointOne,
            _ => panic!("Unsupported version"),
        }
    }
}

impl From<HttpVersion> for &'static str {
    fn from(value: HttpVersion) -> Self {
        match value {
            HttpVersion::OnePointOne => "HTTP/1.1",
        }
    }
}
