use hooch::{hooch_main, net::HoochTcpListener};
use hooch_http::{app::HoochAppBuilder, parser::HttpRequest};

#[hooch_main]
async fn main() {
    let app = HoochAppBuilder::new("localhost:8080").unwrap().build();

    app.serve(handler).await;
}

async fn handler(req: HttpRequest<'_>) {
    println!("REQUEST: {}", req);
}
