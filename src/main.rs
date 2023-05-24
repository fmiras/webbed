use clap::{App, Arg};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, fs};

async fn handle(req: Request<Body>, base_path: PathBuf) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();
    let mut file_path = base_path.clone();

    for part in path.split('/').skip(1) {
        // skip the first empty part
        file_path.push(part);
    }

    let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();

    let response = match fs::read(&file_path) {
        Ok(data) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_type.as_ref())
            .body(Body::from(data))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    };

    Ok(response)
}

fn make_hyper_server(
    base_path: PathBuf,
    addr: SocketAddr,
) -> impl Future<Output = Result<(), hyper::Error>> {
    let make_service = make_service_fn(move |_conn| {
        let base_path = base_path.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle(req, base_path.clone()))) }
    });
    Server::bind(&addr).serve(make_service)
}

#[tokio::main]
async fn main() {
    let matches = App::new("webbed")
        .version("0.1.0")
        .arg(
            Arg::with_name("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the port to use")
                .takes_value(true)
                .default_value("5000"),
        )
        .arg(
            Arg::with_name("directory")
                .value_name("DIRECTORY")
                .help("Sets the directory to serve files from")
                .takes_value(true),
        )
        .get_matches();

    let port: u16 = matches
        .value_of("port")
        .unwrap()
        .parse()
        .expect("Invalid port number");
    let directory = matches.value_of("directory").unwrap_or(".");
    let base_path = env::current_dir().unwrap().join(directory);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let server = make_hyper_server(base_path, addr);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Client;
    use hyper::Uri;
    use lazy_static::lazy_static;
    use std::net::TcpListener;

    lazy_static! {
        static ref TEST_SERVER: SocketAddr = setup_server().expect("Failed to start server");
    }

    // Helper function to find an available port
    fn find_available_port() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
    }

    fn setup_server() -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        let port = find_available_port()?;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let base_path = env::current_dir()?;
        let server = make_hyper_server(base_path, addr);
        tokio::spawn(server);
        Ok(addr)
    }

    #[tokio::test]
    async fn test_fetch_existing_file() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        // Test a file that exists
        let uri = format!(
            "http://{}:{}/Cargo.toml",
            TEST_SERVER.ip(),
            TEST_SERVER.port()
        )
        .parse::<Uri>()?;
        let response = client.get(uri).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_unexisting_file() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        // Test a file that does not exist
        let uri = format!(
            "http://{}:{}/does_not_exist.txt",
            TEST_SERVER.ip(),
            TEST_SERVER.port()
        )
        .parse::<Uri>()?;
        let response = client.get(uri).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
