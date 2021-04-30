use std::{env, net::SocketAddr};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    Ok(Response::new(req.into_body()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!(
        "{host}:{port}",
        host = env::var("SERVER_HOST").unwrap(),
        port = env::var("SERVER_PORT").unwrap(),
    )
    .parse::<SocketAddr>()
    .unwrap();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(echo)) });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
