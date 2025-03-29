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
