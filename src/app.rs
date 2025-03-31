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
    sync::Arc,
};

use hooch::{
    net::{HoochTcpListener, HoochTcpStream},
    spawner::Spawner,
};

use crate::{request::HttpRequest, response::HttpResponse};

#[derive(Debug)]
pub enum Middleware {
    Continue(HttpRequest<'static>),
    CircuitBreak(HttpResponse),
}

/// Builder for configuring and creating a [`HoochApp`] instance.
#[derive(Debug)]
pub struct HoochAppBuilder<Fut, F>
where
    Fut: Future<Output = Middleware>,
    F: Fn(HttpRequest<'static>, SocketAddr) -> Fut + Send + Sync + 'static,
{
    addr: SocketAddr,
    middleware: Vec<F>,
}

impl<Fut, F> HoochAppBuilder<Fut, F>
where
    Fut: Future<Output = Middleware>,
    F: Fn(HttpRequest<'static>, SocketAddr) -> Fut + Send + Sync + 'static,
{
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

    pub fn add_middleware(mut self, middleware: F) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Consumes the builder and returns a [`HoochApp`] instance.
    pub fn build(self) -> HoochApp {
        HoochApp { addr: self.addr }
    }
}

/// A simple HTTP server using the Hooch async runtime.
#[derive(Debug)]
pub struct HoochApp {
    addr: SocketAddr,
    middleware: Arc<Vec<MIDDLEWARE GENERIC>>
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

        while let Ok((stream, socket)) = listener.accept().await {
            println!("Received message from socket {:?}", socket);
            let handler_clone = Arc::clone(&handler);
            Spawner::spawn(async move {
                Self::handle_stream(stream, handler_clone).await;
            });
        }
    }

    /// Handles a single TCP stream, reads the request, and delegates it to the request handler.
    async fn handle_stream<F, Fut>(mut stream: HoochTcpStream, handler: Arc<F>)
    where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let mut buffer = [0; 1024 * 100];
        let bytes_read = stream.read(&mut buffer).await.unwrap();

        let handler_clone = Arc::clone(&handler);
        let http_request = HttpRequest::from_bytes(&buffer[..bytes_read]);

        // SAFETY: We're transmuting to 'static because the request is processed
        // within this async context and the buffer is not used afterward.
        let http_request: HttpRequest<'static> = unsafe { std::mem::transmute(http_request) };

        Self::handle_http_request(http_request, handler_clone, stream).await;
    }

    /// Processes an [`HttpRequest`] using the given handler and writes the resulting [`HttpResponse`] to the stream.
    async fn handle_http_request<F, Fut>(
        http_request: HttpRequest<'static>,
        handler: Arc<F>,
        mut stream: HoochTcpStream,
    ) where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let response = handler(http_request).await;
        let mut buffer = Vec::with_capacity(std::mem::size_of_val(&response));
        buffer = response.serialize(buffer);
        stream.write(&buffer).await.unwrap();
    }
}
