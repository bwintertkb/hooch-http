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

    HttpResponseBuilder::not_found()
        .body("Hello from handler".into())
        .build()
}
