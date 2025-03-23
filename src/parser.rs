const CARRIAGE_RETURN_LINE_FEED: &'static [u8; 2] = b"\r\n";
const CARRIAGE_RETURN_LINE_FEED_TWICE: &'static [u8; 4] = b"\r\n\r\n";
const WHITESPACE_BYTE: u8 = 32;
const COLON_BYTE: u8 = 58;

const MAX_NUM_HEADERS: usize = 1000;

#[derive(Debug)]
pub struct HttpRequestParser<'a> {
    body: &'a [u8],
}

impl<'a> HttpRequestParser<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { body: bytes }
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

    fn get_body(bytes: &[u8]) -> &[u8] {
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

        &bytes[end_of_header_bytes_idx..]
    }

    fn extract_http_method(bytes: &[u8]) -> &[u8] {
        bytes.split(|b| *b == WHITESPACE_BYTE).next().unwrap()
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

            let (key, value) = HttpRequestParser::get_header_key_and_value(
                &bytes[start_idx..carriage_return_idx + start_idx],
            );

            headers.add_key_value(key, value);

            start_idx += carriage_return_idx + CARRIAGE_RETURN_LINE_FEED.len();
        }

        let (key, value) = HttpRequestParser::get_header_key_and_value(&bytes[start_idx..]);
        headers.add_key_value(key, value);

        headers
    }

    fn get_header_key_and_value(bytes: &'a [u8]) -> (&'a str, &'a str) {
        let colon_idx = bytes.iter().position(|byte| *byte == COLON_BYTE).unwrap();
        let key = std::str::from_utf8(&bytes[..colon_idx]);
        /// Need to check if there is a whitespace after the colon
        /// Note true as usize == 1, false as usize == 0
        let whitespace_offset = (bytes[colon_idx + 1] == WHITESPACE_BYTE) as usize;
        let value = std::str::from_utf8(&bytes[colon_idx + 1 + whitespace_offset..]);
        (key.unwrap(), value.unwrap())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Uri<'a>(&'a str);

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
        let actual = HttpRequestParser::get_request_line(&request);

        assert_eq!(actual, expected);
    }

    #[test]
    fn extract_http_method() {
        let request_line = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];

        let expected = [80, 79, 83, 84];

        let actual = HttpRequestParser::extract_http_method(&request_line);

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

        let actual = HttpRequestParser::extract_request_uri(&request_line);
        let expected = Uri("/user");

        assert_eq!(actual, expected);
    }

    #[test]
    fn extract_http_version() {
        let request_line = [
            80, 79, 83, 84, 32, 47, 117, 115, 101, 114, 32, 72, 84, 84, 80, 47, 49, 46, 49,
        ];

        let expected = HttpVersion::OnePointOne;

        let actual = HttpRequestParser::extract_http_version(&request_line);

        assert_eq!(actual, expected);
    }

    #[test]
    fn get_headers() {
        let request = get_test_post_request();

        let actual = HttpRequestParser::get_headers(&request);

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

        let actual = HttpRequestParser::extract_headers(headers);

        let mut expected_keys = [None; MAX_NUM_HEADERS];
        let mut expected_values = [None; MAX_NUM_HEADERS];
        let mut expected_num_headers = 5;

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

        let actual = HttpRequestParser::get_body(&request);

        let expected = b"{\"message\": \"hello world\"}";

        assert_eq!(actual, expected);
    }
}
