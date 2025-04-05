# hooch_http

`hooch_http` is a lightweight, asynchronous HTTP server framework built on top of the [hooch](https://crates.io/crates/hooch) runtime. It provides fast HTTP parsing, simple route matching, support for path/query parameters, and middleware integration—all with zero heap allocations for request parsing.

## ✨ Features

- **Fully Async Server:** Built with `hooch` for scalable, non-blocking I/O.
- **Zero-Allocation Parsing:** Efficiently parses HTTP/1.1 requests including headers and body using direct byte slice manipulation.
- **Route Matching:** Define routes with parameterized URIs (e.g., `/user/{id}`) and extract dynamic segments in a type-safe manner.
- **Query Parameter Extraction:** Easily parse and iterate over query strings.
- **Middleware Support:** Register middleware to intercept, log, or modify requests, or to short-circuit request handling by providing an immediate response.
- **Low-Latency:** Designed for resource-constrained and performance-critical applications.

## 🚀 Example

```rust
use hooch::hooch_main;
use hooch_http::{HoochAppBuilder, HttpMethod, HttpResponseBuilder};

#[hooch_main]
async fn main() {
    let mut app = HoochAppBuilder::new("localhost:8080").unwrap();

    // Example middleware: Log incoming requests.
    app.add_middleware(|req, addr| async move {
        println!("Received request from {}: {:?}", addr, req);
        hooch_http::Middleware::Continue(req)
    });

    // Add a GET route with parameter extraction.
    app.add_route("/what/{mate}", HttpMethod::GET, |req, mut params| async move {
        // Iterate over extracted path parameters.
        for (key, value) in params.iter_path() {
            println!("PATH KEY: {:?}", key);
            println!("PATH VALUE: {:?}", value);
        }
        // Iterate over extracted query parameters.
        for (key, value) in params.iter_query() {
            println!("QUERY KEY: {:?}", key);
            println!("QUERY VALUE: {:?}", value);
        }
        HttpResponseBuilder::ok()
            .body("Hello from inside /what/{mate}".into())
            .build()
    });

    let app = app.build();
    app.serve().await;
}
