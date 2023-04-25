use std::{
    error::Error as StdError,
    fmt,
    net::{AddrParseError, SocketAddr},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use hyper::{client::HttpConnector, http::uri::InvalidUri, Body, Client, Request, Response, Uri};
use tokio::{io::AsyncWriteExt, net::TcpStream, time};

use crate::{Dependency, DependencyWaitError};

pub use hyper::Method as HttpMethod;

const ITER_GAP: Duration = Duration::from_millis(250); // ms

/// Error returned from a network [`Dependency::wait`](Dependency::wait) method.
#[derive(thiserror::Error, Debug)]
enum NetServiceWaitError {
    /// Rejected network request.
    #[error("Rejection: {}", .error)]
    Rejection {
        /// Error from the dependency.
        error: Box<dyn StdError + Send + Sync>,
    },
    /// Request timeout.
    #[error("Timeout")]
    Timeout,
}

impl DependencyWaitError for NetServiceWaitError {}

/// TCP service.
pub struct TcpService {
    /// A tag used as an identificator of the dependency in the output.
    pub tag: String,
    /// Service address.
    pub addr: SocketAddr,
    /// Service wait timeout.
    pub timeout: Duration,
    /// Optional wait time after a successful response from the TCP service.
    pub warm_up: Option<Duration>,
}

impl TcpService {
    /// Consructs new TcpService.
    pub fn new(
        tag: impl Into<String>,
        host: impl fmt::Display,
        port: impl fmt::Display,
        timeout: Duration,
        warm_up: Option<Duration>,
    ) -> Result<Self, AddrParseError> {
        let addr = format!("{}:{}", host, port).parse()?;

        Ok(Self {
            tag: tag.into(),
            addr,
            timeout,
            warm_up,
        })
    }
}

#[async_trait]
impl Dependency for TcpService {
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn check(&self) -> Result<(), ()> {
        match TcpStream::connect(&self.addr).await {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let start = Instant::now();

        loop {
            match time::timeout(
                self.timeout - start.elapsed(),
                TcpStream::connect(&self.addr),
            )
            .await
            {
                Ok(Ok(mut stream)) => {
                    if let Err(error) = stream.shutdown().await {
                        eprintln!("Failed to close socket: {}", error);
                    };

                    if let Some(duration) = self.warm_up {
                        time::sleep(duration).await;
                    }

                    return Ok(());
                }
                Ok(Err(_)) => (),
                Err(_) => {
                    return Err(Box::new(NetServiceWaitError::Timeout));
                }
            }

            if start.elapsed() >= self.timeout {
                return Err(Box::new(NetServiceWaitError::Timeout));
            }

            time::sleep(ITER_GAP).await;
        }
    }
}

/// HTTP service.
pub struct HttpService {
    /// A tag used as an identificator of the dependency in the output.
    pub tag: String,
    /// Service address.
    pub addr: Uri,
    /// HTTP method.
    pub method: HttpMethod,
    /// Service wait timeout.
    pub timeout: Duration,
}

impl HttpService {
    fn http_connector() -> HttpConnector {
        HttpConnector::new()
    }

    #[cfg(feature = "tls")]
    fn https_connector() -> tls::HttpsConnector<HttpConnector> {
        tls::HttpsConnector::new()
    }

    #[cfg(not(feature = "tls"))]
    fn https_connector() -> HttpConnector {
        panic!("Cannot use https_connector method without tls feature");
    }
}

#[derive(Debug)]
struct HttpError {
    status: hyper::StatusCode,
}

impl std::error::Error for HttpError {}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status)
    }
}

impl From<hyper::Response<Body>> for HttpError {
    fn from(res: hyper::Response<Body>) -> Self {
        Self {
            status: res.status(),
        }
    }
}

impl HttpService {
    /// Consructs new HttpService.
    pub fn new(
        tag: impl Into<String>,
        host: impl fmt::Display,
        port: impl fmt::Display,
        path: impl fmt::Display,
        ssl: bool,
        method: HttpMethod,
        timeout: Duration,
    ) -> Result<Self, InvalidUri> {
        let addr = format!(
            "http{}://{}:{}{}",
            if ssl { "s" } else { "" },
            host,
            port,
            path
        )
        .parse()?;

        Ok(Self {
            tag: tag.into(),
            addr,
            method,
            timeout,
        })
    }

    pub(crate) fn build_req(&self) -> Request<Body> {
        Request::builder()
            .method(&self.method)
            .uri(&self.addr)
            .body(Body::default())
            .expect("Failed to build HTTP request")
    }

    fn handle_res(res: Response<Body>) -> Result<(), Box<dyn DependencyWaitError>> {
        if res.status().is_success() {
            Ok(())
        } else {
            Err(Box::new(NetServiceWaitError::Rejection {
                error: Box::new(Into::<HttpError>::into(res)),
            }))
        }
    }
}

#[async_trait]
impl Dependency for HttpService {
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn check(&self) -> Result<(), ()> {
        match self.addr.scheme_str() {
            Some("https") => {
                let connector = Self::https_connector();
                let client = Client::builder().build(connector);
                let req = self.build_req();
                let res = client.request(req).await.map_err(|_| ())?;
                Self::handle_res(res).map_err(|_| ())
            }
            Some(_) | None => {
                let connector = Self::http_connector();
                let client = Client::builder().build(connector);
                let req = self.build_req();
                let res = client.request(req).await.map_err(|_| ())?;
                Self::handle_res(res).map_err(|_| ())
            }
        }
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let start = Instant::now();

        match self.addr.scheme_str() {
            Some("https") => {
                let connector = Self::https_connector();
                let client = Client::builder().build(connector);

                loop {
                    let req = self.build_req();

                    match time::timeout(self.timeout - start.elapsed(), client.request(req)).await {
                        Ok(Ok(res)) => return Self::handle_res(res),
                        Ok(Err(_)) => (),
                        Err(_) => return Err(Box::new(NetServiceWaitError::Timeout)),
                    }

                    if start.elapsed() >= self.timeout {
                        return Err(Box::new(NetServiceWaitError::Timeout));
                    }

                    time::sleep(ITER_GAP).await;
                }
            }
            Some(_) | None => {
                let connector = Self::http_connector();
                let client = Client::builder().build(connector);

                loop {
                    let req = self.build_req();

                    match time::timeout(self.timeout - start.elapsed(), client.request(req)).await {
                        Ok(Ok(res)) => return Self::handle_res(res),
                        Ok(Err(_)) => (),
                        Err(_) => return Err(Box::new(NetServiceWaitError::Timeout)),
                    }

                    if start.elapsed() >= self.timeout {
                        return Err(Box::new(NetServiceWaitError::Timeout));
                    }

                    time::sleep(ITER_GAP).await;
                }
            }
        }
    }
}
