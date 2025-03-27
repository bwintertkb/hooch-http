use std::{
    future::Future,
    io,
    net::{SocketAddr, ToSocketAddrs},
    num::{NonZero, NonZeroUsize},
    sync::Arc,
};

use hooch::{
    net::{HoochTcpListener, HoochTcpStream},
    spawner::Spawner,
};

use crate::{request::HttpRequest, response::HttpResponse};

#[derive(Debug)]
pub struct HoochAppBuilder {
    addr: SocketAddr,
}

impl HoochAppBuilder {
    pub fn new(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no address resolved"))?;

        Ok(Self { addr })
    }

    pub fn build(self) -> HoochApp {
        HoochApp { addr: self.addr }
    }
}

#[derive(Debug)]
pub struct HoochApp {
    addr: SocketAddr,
}

impl HoochApp {
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

    async fn handle_stream<F, Fut>(mut stream: HoochTcpStream, handler: Arc<F>)
    where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let mut buffer = [0; 1024 * 100];
        let bytes_read = stream.read(&mut buffer).await.unwrap();
        println!("BYTES READ: {}", bytes_read);

        let handler_clone = Arc::clone(&handler);
        let http_request = HttpRequest::from_bytes(&buffer[..bytes_read]);
        // This is fine because we move the buffer into the callback, so the lifetime of
        // http_request is the same as the lifetime of the buffer
        let http_request: HttpRequest<'static> = unsafe { std::mem::transmute(http_request) };

        Self::handle_http_request(http_request, handler_clone, stream).await;
    }

    async fn handle_http_request<F, Fut>(
        http_request: HttpRequest<'static>,
        handler: Arc<F>,
        mut stream: HoochTcpStream,
    ) where
        Fut: Future<Output = HttpResponse> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let response = handler(http_request).await;
        println!("Size of value: {}", std::mem::size_of_val(&response));
        let mut buffer = Vec::with_capacity(std::mem::size_of_val(&response));
        buffer = response.serialize(buffer);
        println!("RESPONSE: {:?}", String::from_utf8_lossy(&buffer));
        stream.write(&buffer).await.unwrap();
    }
}
