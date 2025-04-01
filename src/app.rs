//! A simple asynchronous HTTP server framework using the Hooch async runtime.
//!
//! # Example
//!
//! ```rust
//! # use hooch_http::{HoochAppBuilder, HttpRequest, HttpResponseBuilder, HttpResponse};
//! # async fn handler(_req: HttpRequest<'_>) -> HttpResponse {
//! #     HttpResponseBuilder::ok().build()
//! # }
//! # async fn run() {
//! let app = HoochAppBuilder::new("127.0.0.1:8080").unwrap().build();
//! app.serve(handler).await;
//! # }
//! ```

use std::{
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
};

use hooch::{
    net::{HoochTcpListener, HoochTcpStream},
    spawner::Spawner,
};

use crate::{request::HttpRequest, response::HttpResponse};

/// A future that will eventually produce a `Middleware`.
type MiddlewareFuture = Pin<Box<dyn Future<Output = Middleware> + Send>>;

/// A boxed middleware function that takes an HTTP request and socket address and returns a `MiddlewareFuture`.
type MiddlewareFn = Box<dyn Fn(HttpRequest<'static>, SocketAddr) -> MiddlewareFuture + Send + Sync>;

#[derive(Debug)]
pub enum Middleware {
    Continue(HttpRequest<'static>),
    CircuitBreak(HttpResponse),
}

/// Builder for configuring and creating a [`HoochApp`] instance.
pub struct HoochAppBuilder {
    addr: SocketAddr,
    middleware: Vec<MiddlewareFn>,
}

impl HoochAppBuilder {
    /// Creates a new `HoochAppBuilder` from an address that implements [`ToSocketAddrs`].
    ///
    /// # Errors
    ///
    /// Returns an error if the address cannot be resolved.
    pub fn new(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no address resolved"))?;

        Ok(Self {
            addr,
            middleware: Vec::new(),
        })
    }

    pub fn add_middleware<Fut, F>(mut self, middleware: F) -> Self
    where
        Fut: Future<Output = Middleware> + Send + 'static,
        F: Fn(HttpRequest<'static>, SocketAddr) -> Fut + Send + Sync + 'static,
    {
        self.middleware.push(Box::new(move |req, socket| {
            Box::pin(middleware(req, socket))
        }));
        self
    }

    /// Consumes the builder and returns a [`HoochApp`] instance.
    pub fn build(self) -> HoochApp {
        let middleware_ptr: &'static Vec<MiddlewareFn> = Box::leak(Box::new(self.middleware));
        HoochApp {
            addr: self.addr,
            middleware: middleware_ptr,
        }
    }
}

/// A simple HTTP server using the Hooch async runtime.
pub struct HoochApp {
    addr: SocketAddr,
    middleware: &'static Vec<MiddlewareFn>,
}

impl HoochApp {
    /// Starts the HTTP server and begins accepting incoming connections.
    ///
    /// # Arguments
    ///
    /// * `handler` - An asynchronous function or closure that takes an [`HttpRequest`] and returns an [`HttpResponse`].
    pub async fn serve<F, Fut>(&self, handler: F)
    where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let listener = HoochTcpListener::bind(self.addr).await.unwrap();
        let handler = Arc::new(handler);
        let middleware_ptr: &'static Vec<MiddlewareFn> = self.middleware;

        while let Ok((stream, socket)) = listener.accept().await {
            println!("Received message from socket {:?}", socket);
            let handler_clone = Arc::clone(&handler);
            Spawner::spawn(async move {
                Self::handle_stream(stream, socket, handler_clone, middleware_ptr).await;
            });
        }
    }

    /// Handles a single TCP stream, reads the request, and delegates it to the request handler.
    async fn handle_stream<F, Fut>(
        mut stream: HoochTcpStream,
        socket_addr: SocketAddr,
        handler: Arc<F>,
        middleware_fns: &'static [MiddlewareFn],
    ) where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let mut buffer = [0; 1024 * 100];
        let bytes_read = stream.read(&mut buffer).await.unwrap();

        let http_request = HttpRequest::from_bytes(&buffer[..bytes_read]);

        // SAFETY: We're transmuting to 'static because the request is processed
        // within this async context and the buffer is not used afterward.
        let mut http_request: HttpRequest<'static> = unsafe { std::mem::transmute(http_request) };

        for mid in middleware_fns.iter() {
            let middleware = mid(http_request, socket_addr).await;
            match middleware {
                Middleware::Continue(req) => {
                    http_request = req;
                }
                Middleware::CircuitBreak(response) => {
                    return Self::handle_http_response(response, stream).await;
                }
            }
        }

        let response = handler(http_request).await;
        Self::handle_http_response(response, stream).await;
    }

    /// Processes an [`HttpRequest`] using the given handler and writes the resulting [`HttpResponse`] to the stream.
    async fn handle_http_response(http_response: HttpResponse, mut stream: HoochTcpStream) {
        let mut buffer = Vec::with_capacity(std::mem::size_of_val(&http_response));
        buffer = http_response.serialize(buffer);
        stream.write(&buffer).await.unwrap();
    }
}
