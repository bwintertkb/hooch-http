//! HTTP Response Building and Serialization Module
//!
//! This module provides a minimal `HttpResponseBuilder` utility for constructing HTTP
//! responses, along with associated types like `HttpStatus`, `HeaderKey`, and `HeaderValue`.
//! Responses can be serialized into byte buffers for sending over a network.

use std::collections::HashMap;
use std::io::Write;

use crate::shared::HttpVersion;

/// Common HTTP status codes.
#[derive(Debug, Copy, Clone)]
pub enum HttpStatus {
    Ok,
    Created,
    NoContent,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    InternalServerError,
    BadGateway,
    ServiceUnavailable,
}

/// Convert an `HttpStatus` to its numeric status code.
impl From<HttpStatus> for u16 {
    fn from(status: HttpStatus) -> u16 {
        match status {
            HttpStatus::Ok => 200,
            HttpStatus::Created => 201,
            HttpStatus::NoContent => 204,
            HttpStatus::BadRequest => 400,
            HttpStatus::Unauthorized => 401,
            HttpStatus::Forbidden => 403,
            HttpStatus::NotFound => 404,
            HttpStatus::InternalServerError => 500,
            HttpStatus::BadGateway => 502,
            HttpStatus::ServiceUnavailable => 503,
        }
    }
}

/// Convert an `HttpStatus` to its standard reason phrase.
impl From<HttpStatus> for &'static str {
    fn from(status: HttpStatus) -> &'static str {
        match status {
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::NoContent => "No Content",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// Wrapper for HTTP header keys.
#[derive(Debug)]
pub struct HeaderKey(String);

impl AsRef<str> for HeaderKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<String> for HeaderKey {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<String> for HeaderKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Wrapper for HTTP header values.
#[derive(Debug)]
pub struct HeaderValue(String);

impl AsRef<str> for HeaderValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<String> for HeaderValue {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<String> for HeaderValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Builder struct for constructing an HTTP response.
#[derive(Debug)]
pub struct HttpResponseBuilder {
    status: HttpStatus,
    protocal: Option<HttpVersion>,
    headers: Option<HashMap<HeaderKey, HeaderValue>>,
    body: Option<String>,
}

impl HttpResponseBuilder {
    /// Create a builder with a custom status.
    pub fn new(status: HttpStatus) -> Self {
        Self {
            status,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    /// Shortcut for 200 OK.
    pub fn ok() -> Self {
        Self::new(HttpStatus::Ok)
    }

    /// Shortcut for 201 Created.
    pub fn created() -> Self {
        Self::new(HttpStatus::Created)
    }

    /// Shortcut for 204 No Content.
    pub fn no_content() -> Self {
        Self::new(HttpStatus::NoContent)
    }

    /// Shortcut for 400 Bad Request.
    pub fn bad_request() -> Self {
        Self::new(HttpStatus::BadRequest)
    }

    /// Shortcut for 401 Unauthorized.
    pub fn unauthorized() -> Self {
        Self::new(HttpStatus::Unauthorized)
    }

    /// Shortcut for 403 Forbidden.
    pub fn forbidden() -> Self {
        Self::new(HttpStatus::Forbidden)
    }

    /// Shortcut for 404 Not Found.
    pub fn not_found() -> Self {
        Self::new(HttpStatus::NotFound)
    }

    /// Shortcut for 500 Internal Server Error.
    pub fn internal_server_error() -> Self {
        Self::new(HttpStatus::InternalServerError)
    }

    /// Shortcut for 502 Bad Gateway.
    pub fn bad_gateway() -> Self {
        Self::new(HttpStatus::BadGateway)
    }

    /// Shortcut for 503 Service Unavailable.
    pub fn service_unavailable() -> Self {
        Self::new(HttpStatus::ServiceUnavailable)
    }

    /// Set the HTTP protocol version (defaults to 1.1).
    pub fn protocal(mut self, protocal: HttpVersion) -> Self {
        self.protocal = Some(protocal);
        self
    }

    /// Set the response headers.
    pub fn headers(mut self, headers: HashMap<HeaderKey, HeaderValue>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Get a mutable reference to the headers (if present).
    pub fn get_mut_headers(&mut self) -> Option<&mut HashMap<HeaderKey, HeaderValue>> {
        self.headers.as_mut()
    }

    /// Set the response body.
    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    /// Finalize the builder and return a constructed `HttpResponse`.
    pub fn build(self) -> HttpResponse {
        HttpResponse {
            status: self.status,
            protocal: self.protocal.unwrap_or(HttpVersion::OnePointOne),
            headers: self.headers,
            body: self.body,
        }
    }
}

/// Represents a fully built HTTP response.
#[derive(Debug)]
pub struct HttpResponse {
    status: HttpStatus,
    protocal: HttpVersion,
    headers: Option<HashMap<HeaderKey, HeaderValue>>,
    body: Option<String>,
}

impl HttpResponse {
    /// Serialize the HTTP response to a byte buffer, suitable for sending over the network.
    pub fn serialize(self, mut buffer: Vec<u8>) -> Vec<u8> {
        // Write status line
        write!(
            &mut buffer,
            "{} {}\r\n",
            <&str>::from(self.protocal),
            format!("{} {}", u16::from(self.status), <&str>::from(self.status))
        )
        .unwrap();

        // Write headers
        if let Some(headers) = self.headers {
            headers.iter().for_each(|(key, value)| {
                write!(&mut buffer, "{}: {}\r\n", key.as_ref(), value.as_ref()).unwrap();
            });
        }

        // End of headers
        write!(&mut buffer, "\r\n").unwrap();

        // Write body if present
        if let Some(body) = self.body {
            write!(&mut buffer, "{}", body).unwrap();
        }

        buffer
    }
}
