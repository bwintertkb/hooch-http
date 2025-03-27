use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use crate::shared::HttpVersion;

const CARRIAGE_RETURN_LINE_FEED: &[u8; 2] = b"\r\n";
const CARRIAGE_RETURN_LINE_FEED_TWICE: &[u8; 4] = b"\r\n\r\n";
const WHITESPACE_BYTE: u8 = 32;
const COLON_BYTE: u8 = 58;

const MAX_NUM_HEADERS: usize = 1000;

#[derive(Debug)]
pub struct HttpRequest<'a> {
    method: HttpMethod,
    uri: Uri<'a>,
    version: HttpVersion,
    headers: Headers<'a>,
    body: &'a str,
}

unsafe impl Send for HttpRequest<'_> {}

impl Display for HttpRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "Method: {:?}\nUri: {:?}\nVersion: {:?}\nHeaders: {:?}\nBody: {:?}",
            self.method,
            self.uri.0,
            self.version,
            self.headers
                .keys
                .iter()
                .zip(self.headers.values.iter())
                .take(self.headers.num)
                .map(|(key, value)| (key.unwrap(), value.unwrap()))
                .collect::<HashMap<&str, &str>>(),
            self.body
        ))
    }
}

impl<'a> HttpRequest<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        let request_line = Self::get_request_line(bytes);
        let http_method = Self::extract_http_method(request_line);
        let http_version = Self::extract_http_version(request_line);
        let uri = Self::extract_request_uri(request_line);
        let header_bytes = Self::get_headers(bytes);
        let headers = Self::extract_headers(header_bytes);
        let body = Self::get_body(bytes);

        Self {
            method: http_method,
            uri,
            version: http_version,
            headers,
            body,
        }
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    fn get_request_line(bytes: &[u8]) -> &[u8] {
        let idx = bytes
            .windows(CARRIAGE_RETURN_LINE_FEED.len())
            .position(|window| window == CARRIAGE_RETURN_LINE_FEED)
            .unwrap();

        &bytes[..idx]
    }

    fn get_headers(bytes: &[u8]) -> &[u8] {
        let idx = bytes
            .windows(CARRIAGE_RETURN_LINE_FEED.len())
            .position(|window| window == CARRIAGE_RETURN_LINE_FEED)
            .map(|idx| idx + CARRIAGE_RETURN_LINE_FEED.len())
            .unwrap();

        let header_bytes_idx = bytes[idx..]
            .windows(CARRIAGE_RETURN_LINE_FEED_TWICE.len())
            .position(|window| window == CARRIAGE_RETURN_LINE_FEED_TWICE)
            .unwrap();

        &bytes[idx..header_bytes_idx + idx]
    }

    fn get_body(bytes: &[u8]) -> &str {
        let idx = bytes
            .windows(CARRIAGE_RETURN_LINE_FEED.len())
            .position(|window| window == CARRIAGE_RETURN_LINE_FEED)
            .map(|idx| idx + CARRIAGE_RETURN_LINE_FEED.len())
            .unwrap();

        let end_of_header_bytes_idx = bytes[idx..]
            .windows(CARRIAGE_RETURN_LINE_FEED_TWICE.len())
            .position(|window| window == CARRIAGE_RETURN_LINE_FEED_TWICE)
            .map(|header_idx| header_idx + idx + CARRIAGE_RETURN_LINE_FEED_TWICE.len())
            .unwrap();

        std::str::from_utf8(&bytes[end_of_header_bytes_idx..]).unwrap()
    }

    fn extract_http_method(bytes: &[u8]) -> HttpMethod {
        bytes
            .split(|b| *b == WHITESPACE_BYTE)
            .next()
            .unwrap()
            .into()
    }

    fn extract_request_uri(bytes: &[u8]) -> Uri {
        let mut uri_bytes_split = bytes.split(|b| *b == WHITESPACE_BYTE);
        uri_bytes_split.next().unwrap();
        let uri_bytes = uri_bytes_split.next().unwrap();
        Uri(std::str::from_utf8(uri_bytes).unwrap())
    }

    fn extract_http_version(bytes: &[u8]) -> HttpVersion {
        let mut http_version_bytes_split = bytes.split(|b| *b == WHITESPACE_BYTE);
        http_version_bytes_split.next().unwrap();
        http_version_bytes_split.next().unwrap();
        http_version_bytes_split.next().unwrap().into()
    }

    fn extract_headers(bytes: &[u8]) -> Headers {
        let mut headers = Headers::new();
        let mut start_idx = 0;

        loop {
            let Some(carriage_return_idx) = bytes[start_idx..]
                .windows(CARRIAGE_RETURN_LINE_FEED.len())
                .position(|window| window == CARRIAGE_RETURN_LINE_FEED)
            else {
                break;
            };

            let (key, value) = HttpRequest::get_header_key_and_value(
                &bytes[start_idx..carriage_return_idx + start_idx],
            );

            headers.add_key_value(key, value);

            start_idx += carriage_return_idx + CARRIAGE_RETURN_LINE_FEED.len();
        }

        let (key, value) = HttpRequest::get_header_key_and_value(&bytes[start_idx..]);
        headers.add_key_value(key, value);

        headers
    }

    fn get_header_key_and_value(bytes: &'a [u8]) -> (&'a str, &'a str) {
        let colon_idx = bytes.iter().position(|byte| *byte == COLON_BYTE).unwrap();
        let key = std::str::from_utf8(&bytes[..colon_idx]);
        // Need to check if there is a whitespace after the colon
        // Note true as usize == 1, false as usize == 0
        let whitespace_offset = (bytes[colon_idx + 1] == WHITESPACE_BYTE) as usize;
        let value = std::str::from_utf8(&bytes[colon_idx + 1 + whitespace_offset..]);
        (key.unwrap(), value.unwrap())
    }
}

struct Cursor {
    idx: usize,
    start_idx: usize,
}

impl Cursor {
    fn increment(&mut self) {
        self.idx += 1;
    }

    fn get(&self) -> usize {
        self.idx
    }

    fn get_start(&self) -> usize {
        self.start_idx
    }

    fn set_start_to_idx(&mut self) {
        self.start_idx = self.idx;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Uri<'a>(&'a str);

impl AsRef<str> for Uri<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Uri<'_> {
    pub fn is_match(&self, cmp_uri: &str) -> bool {
        let mut start_idx = 0;
        let mut start_idx_uri = 0;
        let mut bracket_hit = false;
        let mut bracket_group_num = 0;
        let mut uri_iter = self.0.chars();
        let mut record_uri_char = false;

        let mut cursor_uri = Cursor {
            idx: 0,
            start_idx: 0,
        };
        let mut cursor_cmp_uri = Cursor {
            idx: 0,
            start_idx: 0,
        };
        for (idx, c) in cmp_uri.char_indices() {
            if record_uri_char {
                println!("[DEBUG] RECORD CHAR {}", c);
                cursor_cmp_uri.set_start_to_idx();
                let mut c_iter = self.0.chars().skip(cursor_uri.get());
                let Some(mut c_uri) = c_iter.next() else {
                    break;
                };
                println!("[DEBUG] RECORD CHAR URI {}", c_uri);
                // THIS IS WHERE VARIABLES WOULD BE CAPTURED
                while c_uri != c {
                    let Some(c_uri_next) = c_iter.next() else {
                        break;
                    };
                    c_uri = c_uri_next;
                    cursor_uri.increment();
                    println!("[DEBUG] RECORD CHAR WITHIN {}", c_uri);
                }
                println!("[DEBUG] FALSE FALSE RECORD CHAR WITHIN {}, {}", c_uri, c);
                cursor_uri.set_start_to_idx();
                record_uri_char = false;
            }

            if c == '{' {
                println!("[DEBUG] BRACKET HIT START");
                println!(
                    "[DEBUG] CMP URI {:?}",
                    &cmp_uri[cursor_cmp_uri.get_start()..cursor_cmp_uri.get()]
                );
                println!(
                    "[DEBUG] SELF 0 URI {:?}",
                    &self.0[cursor_uri.get_start()..cursor_uri.get()]
                );

                if cmp_uri[cursor_cmp_uri.get_start()..cursor_cmp_uri.get()]
                    != self.0[cursor_uri.get_start()..cursor_uri.get()]
                {
                    return false;
                } else {
                    // We need to reset the start index
                    println!("[DEBUG] BRACKET HIT");
                    bracket_hit = true;
                }
            }

            if bracket_hit {
                if c == '}' {
                    println!("[DEBUG] BRACKET HIT END");
                    bracket_hit = false;
                    start_idx = idx + 1;
                    record_uri_char = true;
                    cursor_cmp_uri.set_start_to_idx();
                }
                cursor_cmp_uri.increment();
                continue;
            }

            cursor_uri.increment();
            if !bracket_hit {
                cursor_cmp_uri.increment();
            }
        }

        if start_idx >= cmp_uri.len() {
            return true;
        }

        println!("START INDEX {}", start_idx);
        println!("SELF 0 URI {:?}", &self.0[cursor_uri.get_start()..]);
        println!("CMP URI {:?}", &cmp_uri[cursor_cmp_uri.get_start()..]);

        // Compare the last part
        self.0[cursor_uri.get_start()..] == cmp_uri[cursor_cmp_uri.get_start()..]
    }
}

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
    fn from(value: &[u8]) -> Self {
        match value {
            b"GET" => HttpMethod::GET,
            b"HEAD" => HttpMethod::HEAD,
            b"OPTIONS" => HttpMethod::OPTIONS,
            b"POST" => HttpMethod::POST,
            b"PUT" => HttpMethod::PUT,
            b"PATCH" => HttpMethod::PATCH,
            b"DELETE" => HttpMethod::DELETE,
            _ => panic!(""),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Headers<'a> {
    keys: [Option<&'a str>; MAX_NUM_HEADERS],
    values: [Option<&'a str>; MAX_NUM_HEADERS],
    /// Number of headers
    num: usize,
}

impl Default for Headers<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Headers<'a> {
    pub fn new() -> Self {
        Self {
            keys: [None; MAX_NUM_HEADERS],
            values: [None; MAX_NUM_HEADERS],
            num: 0,
        }
    }

    fn add_key_value(&mut self, key: &'a str, value: &'a str) {
        self.keys[self.num] = Some(key);
        self.values[self.num] = Some(value);
        self.num += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_post_request() -> &'static [u8] {
        b"POST /user HTTP/1.1\r\n\
     Host: localhost:8080\r\n\
     User-Agent: curl/7.81.0\r\n\
     Accept: */*\r\n\
     Content-Type: application/json\r\n\
     Content-Length: 26\r\n\
     \r\n\
     {\"message\": \"hello world\"}"
    }

    #[test]
    fn get_request_line() {
        let request = get_test_post_request();
        // Bytes representing request line
        let expected = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];
        let actual = HttpRequest::get_request_line(request);

        assert_eq!(actual, expected);
    }

    #[test]
    fn extract_http_method() {
        let request_line = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];

        let expected = HttpMethod::POST;

        let actual = HttpRequest::extract_http_method(&request_line);

        assert_eq!(actual, expected);
    }

    #[test]
    fn bytes_to_http_method() {
        let get = [71, 69, 84];
        let actual: HttpMethod = get.as_slice().into();

        assert_eq!(actual, HttpMethod::GET);

        let head = [72, 69, 65, 68];
        let actual: HttpMethod = head.as_slice().into();
        assert_eq!(actual, HttpMethod::HEAD);

        let options = [79, 80, 84, 73, 79, 78, 83];
        let actual: HttpMethod = options.as_slice().into();
        assert_eq!(actual, HttpMethod::OPTIONS);

        let post = [80, 79, 83, 84];
        let actual: HttpMethod = post.as_slice().into();
        assert_eq!(actual, HttpMethod::POST);

        let put = [80, 85, 84];
        let actual: HttpMethod = put.as_slice().into();
        assert_eq!(actual, HttpMethod::PUT);

        let patch = [80, 65, 84, 67, 72];
        let actual: HttpMethod = patch.as_slice().into();
        assert_eq!(actual, HttpMethod::PATCH);

        let delete = [68, 69, 76, 69, 84, 69];
        let actual: HttpMethod = delete.as_slice().into();
        assert_eq!(actual, HttpMethod::DELETE);
    }

    #[test]
    fn extract_request_uri() {
        let request_line = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];

        let actual = HttpRequest::extract_request_uri(&request_line);
        let expected = Uri("/user");

        assert_eq!(actual, expected);
    }

    #[test]
    fn extract_http_version() {
        let request_line = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];

        let expected = HttpVersion::OnePointOne;

        let actual = HttpRequest::extract_http_version(&request_line);

        assert_eq!(actual, expected);
    }

    #[test]
    fn get_headers() {
        let request = get_test_post_request();

        let actual = HttpRequest::get_headers(request);

        let expected = b"Host: localhost:8080\r\n\
     User-Agent: curl/7.81.0\r\n\
     Accept: */*\r\n\
     Content-Type: application/json\r\n\
     Content-Length: 26";

        assert_eq!(actual, expected);
    }

    #[test]
    fn extract_headers() {
        let headers = b"Host: localhost:8080\r\n\
     User-Agent: curl/7.81.0\r\n\
     Accept:*/*\r\n\
     Content-Type:application/json\r\n\
     Content-Length: 26";

        let actual = HttpRequest::extract_headers(headers);

        let mut expected_keys = [None; MAX_NUM_HEADERS];
        let mut expected_values = [None; MAX_NUM_HEADERS];
        let expected_num_headers = 5;

        expected_keys[0] = Some("Host");
        expected_keys[1] = Some("User-Agent");
        expected_keys[2] = Some("Accept");
        expected_keys[3] = Some("Content-Type");
        expected_keys[4] = Some("Content-Length");

        expected_values[0] = Some("localhost:8080");
        expected_values[1] = Some("curl/7.81.0");
        expected_values[2] = Some("*/*");
        expected_values[3] = Some("application/json");
        expected_values[4] = Some("26");

        let expected = Headers {
            keys: expected_keys,
            values: expected_values,
            num: expected_num_headers,
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn get_body() {
        let request = get_test_post_request();

        let actual = HttpRequest::get_body(request);

        let expected = "{\"message\": \"hello world\"}";

        assert_eq!(actual, expected);
    }

    #[test]
    fn uri_is_match_no_parameterized() {
        let uri = Uri("/uri");
        assert!(uri.is_match("/uri"))
    }

    #[test]
    fn ui_is_match_parameterized_single_arg() {
        let uri = Uri("/1244r2");
        assert!(uri.is_match("/{id}"))
    }

    #[test]
    fn ui_is_match_parameterized_multi_arg() {
        let uri = Uri("/orders/123?status=shipped&include=details");
        assert!(uri.is_match("/orders/123?status={statement}&include=details"))
    }

    #[test]
    fn ui_is_match_parameterized_multi_arg2() {
        let uri = Uri("/orders/123?status=shipped&include=details");
        assert!(uri.is_match("/orders/123?status={statement}&{includee}=details"))
    }
}
