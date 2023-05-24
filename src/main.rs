use clap::{App, Arg};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
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

    let make_service = make_service_fn(move |_conn| {
        let base_path = base_path.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle(req, base_path.clone()))) }
    });
    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
