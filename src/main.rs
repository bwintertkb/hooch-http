use hooch::hooch_main;
use hooch_http::{HoochAppBuilder, HttpMethod, HttpResponseBuilder, Middleware};

#[hooch_main]
async fn main() {
    let mut app = HoochAppBuilder::new("localhost:8080").unwrap();

    app.add_middleware(async move |req, addr| {
        println!("Middleware 1, {}", addr);
        Middleware::Continue(req)
    });

    app.add_middleware(async move |req, addr| {
        println!("Middleware 2, {}", addr);
        Middleware::Continue(req)
    });

    app.add_route(
        "/what/{mate}",
        HttpMethod::GET,
        async move |_req, mut params| {
            println!("In path /what/{{mate}}/");
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
            HttpResponseBuilder::ok().build()
        },
    );

    app.add_route("/", HttpMethod::GET, async move |_req, _params| {
        println!("IN ROUTE: /");
        HttpResponseBuilder::ok().build()
    });

    app.build().serve().await;
}
