use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    marker::PhantomData,
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
pub struct PathSegment;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct QuerySegment;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Params<'a> {
    path_segment: Segment<'a, PathSegment>,
    query_fragment: Segment<'a, QuerySegment>,
}

impl Default for Params<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Params<'_> {
    pub fn new() -> Self {
        Self {
            path_segment: Segment::<PathSegment>::new(),
            query_fragment: Segment::<QuerySegment>::new(),
        }
    }

    pub fn path_segment(&self) -> &Segment<'_, PathSegment> {
        &self.path_segment
    }

    pub fn query_segment(&self) -> &Segment<'_, QuerySegment> {
        &self.query_fragment
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Segment<'a, T>
where
    T: Copy + Debug + PartialEq + Eq + Clone,
{
    key: [Option<&'a str>; 1024],
    value: [Option<&'a str>; 1024],
    num: usize,
    iter_cnt: usize,
    marker: PhantomData<T>,
}

impl<'a, T> Segment<'a, T>
where
    T: Copy + Debug + PartialEq + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            key: [None; 1024],
            value: [None; 1024],
            num: 0,
            iter_cnt: 0,
            marker: PhantomData,
        }
    }

    pub fn insert_key_value(&mut self, key: &'a str, value: Option<&'a str>) {
        self.key[self.num] = Some(key);
        self.value[self.num] = value;
        self.num += 1;
    }

    pub fn insert_key(&mut self, key: &'a str) {
        self.key[self.num] = Some(key);
    }

    pub fn insert_value(&mut self, value: Option<&'a str>) {
        self.value[self.num] = value;
        self.num += 1;
    }

    /// Returns number of inserted key value pairs
    pub fn size(&self) -> usize {
        self.num
    }

    pub fn iter(&mut self) -> &mut Self {
        self.iter_cnt = 0;
        self
    }
}

impl<'a> Iterator for Segment<'a, PathSegment> {
    type Item = (Key<'a>, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_cnt >= self.num {
            return None;
        }
        let items = (
            Key(self.key[self.iter_cnt].unwrap()),
            Value(self.value[self.iter_cnt].unwrap()),
        );
        self.iter_cnt += 1;
        Some(items)
    }
}

impl<'a> Iterator for Segment<'a, QuerySegment> {
    type Item = (Key<'a>, Option<Value<'a>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_cnt >= self.num {
            return None;
        }
        let items = (
            Key(self.key[self.iter_cnt].unwrap()),
            self.value[self.iter_cnt].map(Value),
        );
        self.iter_cnt += 1;
        Some(items)
    }
}

#[derive(Debug, PartialEq)]
pub struct Key<'a>(&'a str);

impl AsRef<str> for Key<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[derive(Debug, PartialEq)]
pub struct Value<'a>(&'a str);

impl AsRef<str> for Value<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Uri<'a>(&'a str);

impl AsRef<str> for Uri<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> Uri<'a> {
    pub fn is_match(&self, cmp_uri: &'a str) -> Option<Params<'a>> {
        let mut path_segment = Segment::<PathSegment>::new();

        let mut start_idx = 0;
        let mut bracket_hit = false;
        let mut bracket_hit_idx = 0;
        let mut record_uri_char = false;
        let mut end_with_value = false;

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
                end_with_value = false;
                cursor_cmp_uri.set_start_to_idx();
                let start_idx_uri = cursor_uri.get();
                let mut c_iter = self.0.chars().skip(cursor_uri.get());
                let Some(mut c_uri) = c_iter.next() else {
                    break;
                };
                println!("[DEBUG] RECORD CHAR URI {}", c_uri);
                while c_uri != c {
                    let Some(c_uri_next) = c_iter.next() else {
                        break;
                    };
                    c_uri = c_uri_next;
                    cursor_uri.increment();
                    println!("[DEBUG] RECORD CHAR WITHIN {}", c_uri);
                }
                let end_idx_uri = cursor_uri.get();
                println!("[DEBUG] END URI: {:?}", &self.0[start_idx_uri..end_idx_uri]);
                path_segment.insert_value(Some(&self.0[start_idx_uri..end_idx_uri]));
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
                    return None;
                } else {
                    // We need to reset the start index
                    println!("[DEBUG] BRACKET HIT");
                    bracket_hit = true;
                    bracket_hit_idx = idx;
                }
            }

            if bracket_hit {
                if c == '}' {
                    println!("[DEBUG] BRACKET HIT END");
                    bracket_hit = false;
                    start_idx = idx + 1;
                    record_uri_char = true;
                    end_with_value = true;
                    println!(
                        "[DEBUG] KEY TO ADD {:?}",
                        &cmp_uri[bracket_hit_idx + 1..idx]
                    );
                    path_segment.insert_key(&cmp_uri[bracket_hit_idx + 1..idx]);
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
        let mut params: Option<Params> = None;

        println!("[DEBUG] END WITH VALUE: {}", end_with_value);
        if end_with_value {
            // We need to extract the parameter value
            println!("[DEBUG] END WITH VALUE: {}", end_with_value);
            path_segment.insert_value(Some(&self.0[cursor_uri.get()..]));
        }

        if self.0.contains('?') {
            if let Some(query_fragment) = self.0.split('?').last() {
                let query_segment = Uri::parse_segment(query_fragment);
                params.get_or_insert_default().query_fragment = query_segment;
            }
        }

        if path_segment.num > 0 {
            println!("[DEBUG] WITHIN 2233");
            params.get_or_insert_default().path_segment = path_segment;
        }

        if start_idx >= cmp_uri.len() {
            // Need to check if we end with a value parameter
            println!("[DEBUG] WITHIN");
            return params;
        }

        // Compare the last part
        if self.0[cursor_uri.get_start()..] != cmp_uri[cursor_cmp_uri.get_start()..] {
            println!("WITHIN WITHIN WITHIN");
            return params;
        }

        println!("[DEBUG] BEFORE THE END, but we know it's a match here so we can set it to a default value");
        if params.is_none() {
            params = Some(Params::default());
        }
        params
    }

    fn parse_segment(segment_part: &'a str) -> Segment<'a, QuerySegment> {
        let mut segment: Segment<'a, QuerySegment> = Segment::new();

        segment_part.split('&').for_each(|inner_split| {
            let mut iter = inner_split.splitn(2, '=');
            let Some(key) = iter.next() else {
                return;
            };
            let value = iter.next();
            segment.insert_key_value(key, value);
        });

        segment
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

        let expected_params = Params {
            path_segment: Segment::<PathSegment>::new(),
            query_fragment: Segment::<QuerySegment>::new(),
        };

        assert_eq!(uri.is_match("/uri"), Some(expected_params))
    }

    #[test]
    fn ui_is_match_parameterized_single_arg() {
        let uri = Uri("/1244r2");

        let mut path_segment = Segment::<PathSegment>::new();
        path_segment.key[0] = Some("id");
        path_segment.value[0] = Some("1244r2");
        path_segment.num = 1;

        let expected_params = Params {
            path_segment,
            query_fragment: Segment::<QuerySegment>::new(),
        };
        assert_eq!(uri.is_match("/{id}"), Some(expected_params))
    }

    #[test]
    fn uri_is_match_parameterized_and_query() {
        let uri = Uri("/orders/123?status=hello_matey&include=details");
        let mut path_segment = Segment::<PathSegment>::new();

        path_segment.key[0] = Some("orders_param");
        path_segment.value[0] = Some("orders");
        path_segment.num = 1;

        let mut query_segment = Segment::<QuerySegment>::new();
        query_segment.key[0] = Some("status");
        query_segment.value[0] = Some("hello_matey");
        query_segment.key[1] = Some("include");
        query_segment.value[1] = Some("details");
        query_segment.num = 2;

        let params = Params {
            path_segment,
            query_fragment: query_segment,
        };

        assert_eq!(uri.is_match("/{orders_param}/123"), Some(params))
    }

    #[test]
    fn uri_is_match_parameterized_multi_args() {
        let uri = Uri("/orders/status/123?status=shipped&include=details");

        let mut path_segment = Segment::<PathSegment>::new();
        path_segment.key[0] = Some("orders_param");
        path_segment.value[0] = Some("orders");
        path_segment.key[1] = Some("field");
        path_segment.value[1] = Some("status");
        path_segment.num = 2;

        let mut query_segment = Segment::<QuerySegment>::new();
        query_segment.key[0] = Some("status");
        query_segment.value[0] = Some("shipped");
        query_segment.key[1] = Some("include");
        query_segment.value[1] = Some("details");
        query_segment.num = 2;

        assert_eq!(
            uri.is_match("/{orders_param}/{field}/123"),
            Some(Params {
                path_segment,
                query_fragment: query_segment,
            })
        )
    }

    #[test]
    fn uri_query_segment_params_parsing() {
        let segment = "q=rust&&limit=10&debug&sort=asc";

        let actual = Uri::parse_segment(segment);

        let mut expected = Segment::new();
        expected.key[0] = Some("q");
        expected.key[1] = Some("");
        expected.key[2] = Some("limit");
        expected.key[3] = Some("debug");
        expected.key[4] = Some("sort");
        expected.value[0] = Some("rust");
        expected.value[1] = None;
        expected.value[2] = Some("10");
        expected.value[3] = None;
        expected.value[4] = Some("asc");
        expected.num = 5;

        assert_eq!(actual, expected)
    }

    #[test]
    fn path_segment_iter() {
        let mut path_segment = Segment::<PathSegment>::new();
        path_segment.key[0] = Some("id");
        path_segment.value[0] = Some("123");
        path_segment.key[1] = Some("name");
        path_segment.value[1] = Some("hello");
        path_segment.num = 2;

        let iter = path_segment.iter();

        assert_eq!(iter.next(), Some((Key("id"), Value("123"))));
        assert_eq!(iter.next(), Some((Key("name"), Value("hello"))));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn query_segment_iter() {
        let mut query_segment = Segment::<QuerySegment>::new();
        query_segment.key[0] = Some("id");
        query_segment.value[0] = Some("123");
        query_segment.key[1] = Some("name");
        query_segment.value[1] = None;
        query_segment.num = 2;

        let iter = query_segment.iter();
        assert_eq!(iter.next(), Some((Key("id"), Some(Value("123")))));
        assert_eq!(iter.next(), Some((Key("name"), None)));
        assert_eq!(iter.next(), None);
    }
}
