use std::{
    error::Error as StdError,
    fmt,
    net::SocketAddr,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use hyper::{client::HttpConnector, Body, Client, Request, Response, Uri};
use tokio::{io::AsyncWriteExt, net::TcpStream, time};

use crate::Location;

pub use hyper::Method as HttpMethod;

/// Dependency trait. You can use provided [`TcpDep`](TcpDep), [`HttpDep`](HttpDep), and
/// [`FsDep`](FsDep). Or implement your own (you would need
/// [`async_trait`](https://docs.rs/async-trait/latest/async_trait/)).
#[async_trait]
pub trait Dependency: Send + Sync {
    /// A tag used as an identificator in output when process runs as a part of a [`ProcessPool`](crate::ProcessPool).
    fn tag(&self) -> &str;
    /// A method that resolves when a dependency becomes available.
    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>>;
}

/// Error returned from the [`Dependency::wait`](Dependency::wait) method must implement this trait.
///
/// ```ignore
/// impl DependencyWaitError for MyDependencyWaitError {}
/// ```
pub trait DependencyWaitError: StdError + Send + Sync {}

/// Error returned from a network [`Dependency::wait`](Dependency::wait) method.
#[derive(thiserror::Error, Debug)]
enum NetDependencyWaitError {
    /// Invalid network address.
    #[error("Invalid address: {}. Error: {}", .addr, .error)]
    InvalidAddr {
        /// Network address.
        addr: String,
        /// Parsing address error.
        error: Box<dyn StdError + Send + Sync>,
    },
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

impl DependencyWaitError for NetDependencyWaitError {}

const ITER_GAP: u8 = 250; // ms

/// TCP dependency.
#[derive(Default)]
pub struct TcpDep {
    /// A tag used as an identificator of the dependency in the output.
    pub tag: String,
    /// Host.
    pub host: String,
    /// Port.
    pub port: String,
    /// Optional TCP dependency timeout.
    pub timeout: Option<Duration>,
    /// Optional wait time after a successful response from the TCP dependency.
    pub warm_up: Option<Duration>,
}

#[async_trait]
impl Dependency for TcpDep {
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let addr = format!("{}:{}", self.host, self.port);
        let addr = match addr.parse::<SocketAddr>() {
            Ok(addr) => addr,
            Err(error) => {
                return Err(Box::new(NetDependencyWaitError::InvalidAddr {
                    addr,
                    error: Box::new(error),
                }))
            }
        };
        let expiration = self.timeout.map(|timeout| Instant::now() + timeout);
        loop {
            match TcpStream::connect(addr).await {
                Ok(mut stream) => {
                    if let Err(error) = stream.shutdown().await {
                        eprintln!("Failed to close socket: {}", error);
                    };
                    break;
                }
                Err(_) => {
                    if let Some(expiration) = expiration {
                        if Instant::now() > expiration {
                            return Err(Box::new(NetDependencyWaitError::Timeout));
                        }
                    }
                    time::sleep(Duration::from_millis(ITER_GAP as u64)).await
                }
            }
        }
        if let Some(duration) = self.warm_up {
            time::sleep(duration).await;
        }
        Ok(())
    }
}

/// HTTP dependency.
#[derive(Default)]
pub struct HttpDep {
    ///A tag used as an identificator of the dependency in the output.
    pub tag: String,
    /// Host.
    pub host: String,
    /// Port.
    pub port: String,
    /// Path.
    pub path: String,
    /// Either secure connection or not.
    pub ssl: bool,
    /// HTTP method.
    pub method: HttpMethod,
    /// Optional HTTP dependency timeout.
    pub timeout: Option<Duration>,
}

impl HttpDep {
    fn http_connector() -> HttpConnector {
        HttpConnector::new()
    }

    #[cfg(feature = "tls")]
    fn https_connector() -> tls::HttpsConnector<HttpConnector> {
        tls::HttpsConnector::new()
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

impl HttpDep {
    pub(crate) fn req(&self, uri: Uri) -> Request<Body> {
        Request::builder()
            .method(&self.method)
            .uri(uri)
            .body(Body::default())
            .expect("Failed to build HTTP request")
    }
}

#[async_trait]
impl Dependency for HttpDep {
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let addr = format!(
            "http{}://{}:{}{}",
            if self.ssl { "s" } else { "" },
            self.host,
            self.port,
            self.path
        );
        let addr = match addr.parse::<Uri>() {
            Ok(uri) => uri,
            Err(error) => {
                return Err(Box::new(NetDependencyWaitError::InvalidAddr {
                    addr,
                    error: Box::new(error),
                }))
            }
        };

        let expiration = self.timeout.map(|timeout| Instant::now() + timeout);

        let handle =
            |res: Result<Response<Body>, hyper::Error>| -> Option<Result<(), Box<dyn DependencyWaitError>>> {
                match res {
                    Ok(res) => {
                        if res.status().is_success() {
                            Some(Ok(()))
                        } else {
                            Some(Err(Box::new(NetDependencyWaitError::Rejection {
                                error: Box::new(Into::<HttpError>::into(res)),
                            })))
                        }
                    }
                    Err(_) => {
                        if let Some(expiration) = expiration {
                            if Instant::now() > expiration {
                                return Some(Err(Box::new(NetDependencyWaitError::Timeout)));
                            }
                        }
                        None
                    }
                }
            };

        if self.ssl {
            let connector = Self::https_connector();
            let client = Client::builder().build(connector);
            loop {
                let req = self.req(addr.clone());
                match handle(client.request(req).await) {
                    Some(res) => return res,
                    None => time::sleep(Duration::from_millis(ITER_GAP as u64)).await,
                }
            }
        } else {
            let connector = Self::http_connector();
            let client = Client::builder().build(connector);
            loop {
                let req = self.req(addr.clone());
                match handle(client.request(req).await) {
                    Some(res) => return res,
                    None => time::sleep(Duration::from_millis(ITER_GAP as u64)).await,
                }
            }
        }
    }
}

/// File system dependency.
pub struct FsDep<Loc> {
    /// A tag used as an identificator of the FS dependency in the output.
    pub tag: String,
    /// A path to the FS dependency.
    pub path: Loc,
    /// Optional FS dependency timeout.
    pub timeout: Option<Duration>,
}

#[derive(thiserror::Error, Debug)]
enum FsDependencyWaitError {
    #[error("Timeout")]
    Timeout,
}

impl DependencyWaitError for FsDependencyWaitError {}

#[async_trait]
impl<Loc> Dependency for FsDep<Loc>
where
    Loc: Location,
{
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let path = self.path.as_path();
        let expiration = self.timeout.map(|timeout| Instant::now() + timeout);
        loop {
            if path.exists() {
                break;
            } else {
                if let Some(expiration) = expiration {
                    if Instant::now() > expiration {
                        return Err(Box::new(FsDependencyWaitError::Timeout));
                    }
                }
                time::sleep(Duration::from_millis(ITER_GAP as u64)).await
            }
        }

        Ok(())
    }
}
