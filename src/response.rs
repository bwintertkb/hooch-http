use std::collections::HashMap;
use std::io::Write;

use crate::shared::HttpVersion;

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

#[derive(Debug)]
pub struct HttpResponseBuilder {
    status: HttpStatus,
    protocal: Option<HttpVersion>,
    headers: Option<HashMap<HeaderKey, HeaderValue>>,
    body: Option<String>,
}

impl HttpResponseBuilder {
    pub fn new(status: HttpStatus) -> Self {
        Self {
            status,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn ok() -> Self {
        Self {
            status: HttpStatus::Ok,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn created() -> Self {
        Self {
            status: HttpStatus::Created,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn no_content() -> Self {
        Self {
            status: HttpStatus::NoContent,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn bad_request() -> Self {
        Self {
            status: HttpStatus::BadRequest,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: HttpStatus::Unauthorized,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            status: HttpStatus::Forbidden,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: HttpStatus::NotFound,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn internal_server_error() -> Self {
        Self {
            status: HttpStatus::InternalServerError,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn bad_gateway() -> Self {
        Self {
            status: HttpStatus::BadGateway,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn service_unavailable() -> Self {
        Self {
            status: HttpStatus::ServiceUnavailable,
            protocal: None,
            headers: None,
            body: None,
        }
    }

    pub fn protocal(mut self, protocal: HttpVersion) -> Self {
        self.protocal = Some(protocal);
        self
    }

    pub fn headers(mut self, headers: HashMap<HeaderKey, HeaderValue>) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn get_mut_headers(&mut self) -> Option<&mut HashMap<HeaderKey, HeaderValue>> {
        self.headers.as_mut()
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn build(self) -> HttpResponse {
        HttpResponse {
            status: self.status,
            protocal: self.protocal.unwrap_or(HttpVersion::OnePointOne),
            headers: self.headers,
            body: self.body,
        }
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    status: HttpStatus,
    protocal: HttpVersion,
    headers: Option<HashMap<HeaderKey, HeaderValue>>,
    body: Option<String>,
}

impl HttpResponse {
    pub fn serialize(self, mut buffer: Vec<u8>) -> Vec<u8> {
        write!(
            &mut buffer,
            "{} {} {}\r\n",
            <&str>::from(self.protocal),
            u16::from(self.status),
            <&str>::from(self.status)
        )
        .unwrap();

        if let Some(headers) = self.headers {
            headers.iter().for_each(|(key, value)| {
                write!(&mut buffer, "{}: {}\r\n", key.as_ref(), value.as_ref()).unwrap();
            });
        }

        write!(&mut buffer, "\r\n").unwrap();

        if let Some(body) = self.body {
            write!(&mut buffer, "{}", body).unwrap();
        }

        buffer
    }
}
