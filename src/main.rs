use hooch::{hooch_main, net::HoochTcpListener};
use hooch_http::{
    app::HoochAppBuilder,
    request::HttpRequest,
    response::{HttpResponse, HttpResponseBuilder},
};

#[hooch_main]
async fn main() {
    let app = HoochAppBuilder::new("localhost:8080").unwrap().build();

    app.serve(handler).await;
}

async fn handler(req: HttpRequest<'_>) -> HttpResponse {
    println!("REQUEST: {}", req);

    if let Some(params) = req.uri().is_match("/what/mate") {
        return HttpResponseBuilder::ok()
            .body("Hello from inside what mate".into())
            .build();
    }

    HttpResponseBuilder::not_found()
        .body("Hello from handler".into())
        .build()
}
