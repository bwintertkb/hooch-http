//! Defines the supported HTTP versions and conversion logic.
//!
//! This module provides the `HttpVersion` enum to represent HTTP protocol versions
//! and implements conversions from raw bytes and to string representations.

/// Represents the supported HTTP protocol versions.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP version 1.1
    OnePointOne,
}

impl From<&[u8]> for HttpVersion {
    /// Converts a byte slice into an `HttpVersion` enum.
    ///
    /// # Panics
    /// Panics if the provided byte slice does not correspond to a supported HTTP version.
    fn from(value: &[u8]) -> Self {
        match value {
            b"HTTP/1.1" => HttpVersion::OnePointOne,
            _ => panic!("Unsupported version"),
        }
    }
}

impl From<HttpVersion> for &'static str {
    /// Converts an `HttpVersion` enum into its corresponding string representation.
    fn from(value: HttpVersion) -> Self {
        match value {
            HttpVersion::OnePointOne => "HTTP/1.1",
        }
    }
}

/// Supported HTTP methods.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    HEAD,
    OPTIONS,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl From<&[u8]> for HttpMethod {
    /// Convert raw bytes (e.g., b"GET") to an `HttpMethod` enum variant.
    fn from(value: &[u8]) -> Self {
        match value {
            b"GET" => HttpMethod::GET,
            b"HEAD" => HttpMethod::HEAD,
            b"OPTIONS" => HttpMethod::OPTIONS,
            b"POST" => HttpMethod::POST,
            b"PUT" => HttpMethod::PUT,
            b"PATCH" => HttpMethod::PATCH,
            b"DELETE" => HttpMethod::DELETE,
            _ => panic!("Unknown HTTP method"),
        }
    }
}
