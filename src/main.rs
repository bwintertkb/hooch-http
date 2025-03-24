use hooch::{hooch_main, net::HoochTcpListener};
use hooch_http::parser::HttpRequestParser;

#[hooch_main]
async fn main() {
    let listener = HoochTcpListener::bind("localhost:8080").await.unwrap();
    println!("Running listener, waiting for connections...");
    while let Ok((mut stream, socket)) = listener.accept().await {
        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).await.unwrap();

        let http_request = HttpRequestParser::from_bytes(&buffer[..bytes_read]);
        println!("HTTP REQUEST: {}", http_request);
    }
}
