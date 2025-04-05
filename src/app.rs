//! # Hooch HTTP Server Application
//!
//! This module implements a simple asynchronous HTTP server built on the Hooch async runtime.
//! It supports customizable middleware and routing, enabling developers to process HTTP requests
//! and dispatch them to appropriate handlers based on URI patterns and HTTP methods.
//!
//! ## Features
//!
//! - **Middleware Support:**  
//!   Middleware functions can be registered to process incoming HTTP requests. They can modify
//!   requests or short-circuit further processing by returning an immediate HTTP response.
//!
//! - **Routing:**  
//!   Routes can be defined with parameterized URI patterns and HTTP method matching. The router
//!   matches incoming requests to routes and invokes the corresponding asynchronous handler.
//!
//! - **Asynchronous I/O:**  
//!   The server uses `HoochTcpListener` and `HoochTcpStream` to handle TCP connections asynchronously,
//!   ensuring scalable and non-blocking I/O operations.
//!
//! - **Static Lifetime Management:**  
//!   Middleware and route handlers are required to have a `'static` lifetime. To satisfy this, the
//!   middleware and route vectors are leaked during the build process.
//!
//! ## Usage
//!
//! Use the [`HoochAppBuilder`] to configure the server's address, middleware, and routes. Once
//! configured, call the `build` method to create a [`HoochApp`] instance, and then invoke its `serve`
//! method to start accepting connections.
//!
//! ### Example
//!
//! ```rust
//! use hooch_http::{HoochAppBuilder, HttpResponseBuilder, HttpMethod, Middleware};
//!
//! # async {
//! let mut app = HoochAppBuilder::new("127.0.0.1:8080").unwrap();
//!
//! // Add middleware that logs incoming requests
//! app.add_middleware(|req, socket| async move {
//!     println!("Incoming request from {}: {:?}", socket, req);
//!     Middleware::Continue(req)
//! });
//!
//! // Add a simple GET route for "/hello"
//! app.add_route("/hello", HttpMethod::GET, |req, params| async move {
//!     HttpResponseBuilder::ok().body("Hello, world!".to_string()).build()
//! });
//!
//! let app = app.build();
//! app.serve().await;
//! # };
//! ```
//!
//! This module is ideal for applications requiring a lightweight, customizable HTTP server with minimal
//! runtime dependencies and asynchronous processing.
use std::{
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
};

use futures::FutureExt;

use hooch::{
    net::{HoochTcpListener, HoochTcpStream},
    spawner::Spawner,
};

use crate::{
    request::HttpRequest, response::HttpResponse, HttpMethod, HttpResponseBuilder, Params, Uri,
};

/// A future that will eventually resolve to a [`Middleware`] result.
type MiddlewareFuture = Pin<Box<dyn Future<Output = Middleware> + Send>>;

/// A boxed middleware function. It takes an HTTP request and the client's socket address,
/// and returns a [`MiddlewareFuture`] that resolves to either a modified request or a short-circuited response.
type MiddlewareFn = Box<dyn Fn(HttpRequest<'static>, SocketAddr) -> MiddlewareFuture + Send + Sync>;

/// A future that will eventually resolve to an [`HttpResponse`].
type RouterFuture = Pin<Box<dyn Future<Output = HttpResponse> + Send>>;

/// A boxed router function. It accepts an HTTP request and route parameters,
/// and returns a [`RouterFuture`] resolving to an [`HttpResponse`].
type RouterFn = Box<dyn Fn(HttpRequest<'static>, Params<'static>) -> RouterFuture + Send + Sync>;

/// Enum representing the outcome of middleware processing.
#[derive(Debug)]
pub enum Middleware {
    /// Continue processing the request, possibly with modifications.
    Continue(HttpRequest<'static>),
    /// Short-circuit further processing by immediately returning this response.
    ShortCircuit(HttpResponse),
}

/// Structure representing a single route with its associated HTTP method, path, and handler.
pub struct Route {
    /// The asynchronous handler function for this route.
    fut: RouterFn,
    /// The HTTP method that this route responds to.
    method: HttpMethod,
    /// The URI pattern against which requests are matched.
    path: &'static str,
}

/// Builder for configuring and creating a [`HoochApp`] instance.
///
/// The builder collects middleware and routes, then consumes itself to create a static instance
/// of the application. Note that middleware and routes are leaked to achieve a `'static` lifetime.
pub struct HoochAppBuilder {
    addr: SocketAddr,
    middleware: Vec<MiddlewareFn>,
    router: Vec<Route>,
}

impl HoochAppBuilder {
    /// Creates a new `HoochAppBuilder` from an address that implements [`ToSocketAddrs`].
    ///
    /// # Errors
    ///
    /// Returns an error if the provided address cannot be resolved.
    pub fn new(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no address resolved"))?;

        Ok(Self {
            addr,
            middleware: Vec::new(),
            router: Vec::new(),
        })
    }

    /// Adds a middleware function to the application.
    ///
    /// The middleware is a function that receives an HTTP request and the client's socket address,
    /// and returns a [`MiddlewareFuture`] indicating whether to continue processing or short-circuit.
    pub fn add_middleware<Fut, F>(&mut self, middleware: F)
    where
        Fut: Future<Output = Middleware> + Send + 'static,
        F: Fn(HttpRequest<'static>, SocketAddr) -> Fut + Send + Sync + 'static,
    {
        self.middleware.push(Box::new(move |req, socket| {
            Box::pin(middleware(req, socket))
        }));
    }

    /// Adds a new route to the application.
    ///
    /// The route is specified by a URI pattern, an HTTP method, and a handler function.
    /// The handler receives the request and extracted route parameters, and returns a [`RouterFuture`].
    pub fn add_route<FutRoute, FnRoute>(
        &mut self,
        path: &'static str,
        method: HttpMethod,
        route: FnRoute,
    ) where
        FnRoute: Fn(HttpRequest<'static>, Params<'static>) -> FutRoute + Sync + Send + 'static,
        FutRoute: Future<Output = HttpResponse> + Send + 'static,
    {
        let route = Route {
            fut: Box::new(move |req, params| route(req, params).boxed()),
            method,
            path,
        };
        self.router.push(route);
    }

    /// Consumes the builder and returns a [`HoochApp`] instance.
    ///
    /// This function leaks the middleware and route vectors in order to provide them with a `'static` lifetime,
    /// which is required by the async runtime.
    pub fn build(self) -> HoochApp {
        let middleware_ptr: &'static Vec<MiddlewareFn> = Box::leak(Box::new(self.middleware));
        let route_ptr: &'static Vec<Route> = Box::leak(Box::new(self.router));
        HoochApp {
            addr: self.addr,
            middleware: middleware_ptr,
            routes: route_ptr,
        }
    }
}

/// A simple HTTP server built on the Hooch async runtime.
///
/// `HoochApp` listens for incoming TCP connections, processes HTTP requests through a series of middleware,
/// matches requests to routes, and returns serialized HTTP responses.
pub struct HoochApp {
    addr: SocketAddr,
    middleware: &'static Vec<MiddlewareFn>,
    routes: &'static Vec<Route>,
}

impl HoochApp {
    /// Starts the HTTP server and begins accepting incoming connections.
    ///
    /// The server binds to the configured address, and for each accepted connection,
    /// it spawns an asynchronous task to handle the stream.
    pub async fn serve(&self) {
        let listener = HoochTcpListener::bind(self.addr).await.unwrap();
        let middleware_ptr: &'static Vec<MiddlewareFn> = self.middleware;
        let route_ptr: &'static Vec<Route> = self.routes;

        while let Ok((stream, socket)) = listener.accept().await {
            println!("Received connection from {:?}", socket);
            Spawner::spawn(async move {
                Self::handle_stream(stream, socket, middleware_ptr, route_ptr).await;
            });
        }
    }

    /// Handles a single TCP stream.
    ///
    /// This method reads the HTTP request from the stream, applies all middleware in sequence,
    /// and then routes the request to the appropriate handler based on HTTP method and URI matching.
    /// If a middleware short-circuits the processing or no matching route is found, an appropriate
    /// HTTP response is sent back immediately.
    ///
    /// # Arguments
    ///
    /// * `stream` - The TCP stream representing the client connection.
    /// * `socket_addr` - The client's socket address.
    /// * `middleware_fns` - A slice of middleware functions to process the request.
    /// * `routes` - A slice of defined routes to match against the request.
    async fn handle_stream(
        mut stream: HoochTcpStream,
        socket_addr: SocketAddr,
        middleware_fns: &'static [MiddlewareFn],
        routes: &'static [Route],
    ) {
        let mut buffer = [0; 1024 * 100];
        let bytes_read = stream.read(&mut buffer).await.unwrap();

        // Parse the raw bytes into an HTTP request.
        let http_request = HttpRequest::from_bytes(&buffer[..bytes_read]);

        // SAFETY: Transmute the lifetime of the request to 'static since the buffer is no longer used.
        let mut http_request: HttpRequest<'static> = unsafe { std::mem::transmute(http_request) };

        // Process middleware sequentially. If any middleware returns a ShortCircuit,
        // send its response immediately without further processing.
        for mid in middleware_fns.iter() {
            let middleware = mid(http_request, socket_addr).await;
            match middleware {
                Middleware::Continue(req) => {
                    http_request = req;
                }
                Middleware::ShortCircuit(response) => {
                    return Self::handle_http_response(response, stream).await;
                }
            }
        }

        // Iterate through routes to find a match for the request's URI and HTTP method.
        for route in routes.iter() {
            // SAFETY: Transmute the URI lifetime to 'static for matching within this async context.
            let uri: &Uri<'static> = unsafe { std::mem::transmute(http_request.uri()) };
            if let Some(param) = uri.is_match(route.path) {
                if route.method == http_request.method() {
                    let response = (route.fut)(http_request, param).await;
                    return Self::handle_http_response(response, stream).await;
                }
            }
        }

        // If no matching route is found, respond with a 404 Not Found.
        Self::handle_http_response(HttpResponseBuilder::not_found().build(), stream).await;
    }

    /// Serializes an [`HttpResponse`] and writes it to the TCP stream.
    ///
    /// This method converts the response into a byte vector and writes it to the stream,
    /// sending the complete HTTP response back to the client.
    ///
    /// # Arguments
    ///
    /// * `http_response` - The response to serialize and send.
    /// * `stream` - The TCP stream to write the response to.
    async fn handle_http_response(http_response: HttpResponse, mut stream: HoochTcpStream) {
        let mut buffer = Vec::with_capacity(std::mem::size_of_val(&http_response));
        buffer = http_response.serialize(buffer);
        stream.write(&buffer).await.unwrap();
    }
}
