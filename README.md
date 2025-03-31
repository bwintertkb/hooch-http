# hooch_http

`hooch_http` is a lightweight, async HTTP server framework built on top of the [`hooch`](https://crates.io/crates/hooch) runtime. It provides fast HTTP parsing, simple route matching, and support for path/query parameters—all with zero allocations for request parsing.

## ✨ Features

- Fully async server built with `hooch`
- Request parsing with support for:
  - HTTP/1.1
  - Headers
  - Request body
- Path and query parameter extraction
- Lightweight and minimal boilerplate
- Designed for low-latency use cases

## 🚀 Example

```rust
use hooch::hooch_main;
use hooch_http::{HoochAppBuilder, HttpRequest, HttpResponse, HttpResponseBuilder};

#[hooch_main]
async fn main() {
    let app = HoochAppBuilder::new("localhost:8080").unwrap().build();
    app.serve(handler).await;
}

async fn handler(req: HttpRequest<'_>) -> HttpResponse {
    if let Some(mut params) = req.uri().is_match("/what/{mate}") {
        let iter_path = params.iter_path();
        for (key, value) in iter_path.by_ref() {
            println!("PATH KEY: {:?}", key);
            println!("PATH VALUE: {:?}", value);
        }

        let iter_query = params.iter_query();
        for (key, value) in iter_query.by_ref() {
            println!("QUERY KEY: {:?}", key);
            println!("QUERY VALUE: {:?}", value);
        }

        return HttpResponseBuilder::ok()
            .body("Hello from inside what mate".into())
            .build();
    }

    HttpResponseBuilder::not_found()
        .body("Hello from handler".into())
        .build()
}
```

### 🧪 Try it

Start the server:

```bash
cargo run
```

Then, test it:

```bash
curl "http://localhost:8080/what/123?this=that"
```

Console output:

```
PATH KEY: Key("mate")
PATH VALUE: Value("123")
QUERY KEY: Key("this")
QUERY VALUE: Some(Value("that"))
```

## 📚 Documentation

- Requests are parsed using efficient zero-copy logic.
- You can match URIs with placeholders like `/user/{id}`.
- Query strings are also parsed and exposed via `.iter_query()`.

## 🔒 Safety

All unsafe blocks are carefully justified and used to optimize buffer reuse and lifetimes. No unsafe is used where avoidable.

## 🛠️ TODO

- Add support for middleware
- Routing table
- TLS support
- Request timeouts and keep-alive

## 📄 License

Apache-2.0 

---

Happy hacking! 🚀
