use std::{env, net::SocketAddr, time::Duration};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use tokio::time;

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    Ok(Response::new(req.into_body()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Delaying a start to be able to demo dependent processes
    time::sleep(Duration::from_secs(5)).await;

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
