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

use crate::parser::HttpRequest;

#[derive(Debug)]
pub struct HoochAppBuilder {
    addr: SocketAddr,
    workers: NonZero<usize>,
}

impl HoochAppBuilder {
    pub fn new(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no address resolved"))?;

        Ok(Self {
            addr,
            workers: std::thread::available_parallelism().unwrap(),
        })
    }

    pub fn workers(mut self, workers: NonZero<usize>) -> Self {
        self.workers = workers;
        self
    }

    pub fn build(self) -> HoochApp {
        HoochApp {
            addr: self.addr,
            workers: self.workers,
        }
    }
}

#[derive(Debug)]
pub struct HoochApp {
    addr: SocketAddr,
    workers: NonZero<usize>,
}

impl HoochApp {
    pub async fn serve<F, Fut>(&self, handler: F)
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        let listener = HoochTcpListener::bind(self.addr).await.unwrap();

        let handler = Arc::new(handler);

        while let Ok((stream, socket)) = listener.accept().await {
            println!("Received message from socket {:?}", socket);
            let handler_clone = Arc::clone(&handler);
            println!("handler clone");
            Spawner::spawn(async move {
                println!("HTTP 22");
                Self::handle_stream(stream, handler_clone).await;
            });
        }
    }

    async fn handle_stream<F, Fut>(mut stream: HoochTcpStream, handler: Arc<F>)
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        loop {
            let mut buffer = [0; 1024 * 100];
            let bytes_read = stream.read(&mut buffer).await.unwrap();
            println!("BYTES READ: {}", bytes_read);

            let handler_clone = Arc::clone(&handler);
            Spawner::spawn(async move {
                let http_request = HttpRequest::from_bytes(&buffer[..bytes_read]);
                let http_request: HttpRequest<'static> =
                    unsafe { std::mem::transmute(http_request) };

                Self::handle_http_request(http_request, handler_clone).await;
            });
        }
    }

    async fn handle_http_request<F, Fut>(http_request: HttpRequest<'static>, handler: Arc<F>)
    where
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(HttpRequest<'static>) -> Fut + Send + Sync + 'static,
    {
        handler(http_request).await;
    }
}
