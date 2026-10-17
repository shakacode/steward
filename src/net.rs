use std::{
    error::Error as StdError,
    fmt,
    net::{AddrParseError, SocketAddr},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{uri::InvalidUri, Request, Response, Uri};
use http_body_util::Empty;
use hyper_util::{
    client::legacy::{connect::Connect, Client},
    rt::TokioExecutor,
};
use tokio::{io::AsyncWriteExt, net::TcpStream, time};

use crate::{Dependency, DependencyWaitError};

pub use http::Method as HttpMethod;

const ITER_GAP: Duration = Duration::from_millis(250);

#[derive(thiserror::Error, Debug)]
enum NetServiceWaitError {
    #[error("Rejection: {}", .error)]
    Rejection {
        error: Box<dyn StdError + Send + Sync>,
    },
    #[error("Timeout")]
    Timeout,
}

impl DependencyWaitError for NetServiceWaitError {}

/// TCP service.
#[derive(Debug)]
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
        tag: impl fmt::Display,
        host: impl fmt::Display,
        port: impl fmt::Display,
        timeout: Duration,
        warm_up: Option<Duration>,
    ) -> Result<Self, AddrParseError> {
        let addr = format!("{host}:{port}").parse()?;

        Ok(Self {
            tag: tag.to_string(),
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
            let remaining = self.timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(Box::new(NetServiceWaitError::Timeout));
            }

            match time::timeout(remaining, TcpStream::connect(&self.addr)).await {
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

            time::sleep(ITER_GAP).await;
        }
    }
}

/// HTTP service.
#[derive(Debug)]
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

type EmptyBody = Empty<Bytes>;

/// Builds an HTTP client with the provided connector.
fn build_client<C>(connector: C) -> Client<C, EmptyBody>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    Client::builder(TokioExecutor::new()).build(connector)
}

impl HttpService {
    /// Returns a plain HTTP connector.
    fn http_connector() -> hyper_util::client::legacy::connect::HttpConnector {
        hyper_util::client::legacy::connect::HttpConnector::new()
    }

    #[cfg(feature = "tls")]
    /// Returns an HTTPS connector with TLS support.
    fn https_connector() -> tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>
    {
        tls::HttpsConnector::new()
    }

    #[cfg(not(feature = "tls"))]
    /// Returns a plain HTTP connector (TLS feature disabled).
    fn https_connector() -> hyper_util::client::legacy::connect::HttpConnector {
        unreachable!("Cannot use https_connector method without tls feature")
    }
}

#[derive(Debug)]
struct HttpError {
    status: http::StatusCode,
}

impl std::error::Error for HttpError {}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status)
    }
}

impl HttpService {
    /// Consructs new HttpService.
    pub fn new(
        tag: impl fmt::Display,
        host: impl fmt::Display,
        port: impl fmt::Display,
        path: impl fmt::Display,
        ssl: bool,
        method: HttpMethod,
        timeout: Duration,
    ) -> Result<Self, InvalidUri> {
        let addr = format!("http{}://{host}:{port}{path}", if ssl { "s" } else { "" }).parse()?;

        Ok(Self {
            tag: tag.to_string(),
            addr,
            method,
            timeout,
        })
    }

    /// Constructs an HTTP request for the dependency check.
    pub(crate) fn build_req(&self) -> Result<Request<EmptyBody>, http::Error> {
        Request::builder()
            .method(&self.method)
            .uri(&self.addr)
            .body(EmptyBody::new())
    }

    /// Checks the HTTP response and returns an error for non-success status codes.
    fn handle_res(
        res: &Response<hyper::body::Incoming>,
    ) -> Result<(), Box<dyn DependencyWaitError>> {
        if res.status().is_success() {
            Ok(())
        } else {
            Err(Box::new(NetServiceWaitError::Rejection {
                error: Box::new(HttpError {
                    status: res.status(),
                }),
            }))
        }
    }

    /// Returns true if the service address uses the `https` scheme.
    fn is_https(&self) -> bool {
        matches!(self.addr.scheme_str(), Some("https"))
    }

    /// Performs a single dependency check using the provided connector.
    async fn check_with<C>(&self, connector: C) -> Result<(), ()>
    where
        C: Connect + Clone + Send + Sync + 'static,
    {
        let client = build_client(connector);
        let req = self.build_req().map_err(|_| ())?;
        let res = client.request(req).await.map_err(|_| ())?;
        Self::handle_res(&res).map_err(|_| ())
    }

    /// Polls the HTTP service until it responds successfully or the timeout expires.
    async fn wait_with<C>(&self, connector: C) -> Result<(), Box<dyn DependencyWaitError>>
    where
        C: Connect + Clone + Send + Sync + 'static,
    {
        let client = build_client(connector);
        let start = Instant::now();

        loop {
            let remaining = self.timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(Box::new(NetServiceWaitError::Timeout));
            }

            let req = match self.build_req() {
                Ok(req) => req,
                Err(err) => {
                    return Err(Box::new(NetServiceWaitError::Rejection {
                        error: Box::new(err),
                    }));
                }
            };

            match time::timeout(remaining, client.request(req)).await {
                Ok(Ok(res)) => return Self::handle_res(&res),
                Ok(Err(_)) => (),
                Err(_) => return Err(Box::new(NetServiceWaitError::Timeout)),
            }

            time::sleep(ITER_GAP).await;
        }
    }
}

#[async_trait]
impl Dependency for HttpService {
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn check(&self) -> Result<(), ()> {
        if self.is_https() {
            self.check_with(Self::https_connector()).await
        } else {
            self.check_with(Self::http_connector()).await
        }
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        if self.is_https() {
            self.wait_with(Self::https_connector()).await
        } else {
            self.wait_with(Self::http_connector()).await
        }
    }
}
