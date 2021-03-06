mod request;
mod response;

use clap::Parser;
use rand::{Rng, SeedableRng};
// use std::net::{TcpListener, TcpStream};
use std::io;
use tokio::net::{TcpListener, TcpStream};

use tokio::sync::Mutex;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::delay_for;
use std::collections::HashMap;

const SECONDS_PER_MINUTE: u64 = 60;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Parser, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        help = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, help = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        help = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
    long,
    help = "Path to send request to for active health checks",
    default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        help = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
#[derive(Debug)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
    /// (activate_num, activate_vec)
    activate_addresses: Mutex<(usize, Vec<bool>)>,
    /// ratio limiting
    ratio_limit: Mutex<HashMap<String, usize>>
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    let init_activate_num = options.upstream.len();
    // Handle incoming connections
    let state = Arc::new(ProxyState {
        upstream_addresses: options.upstream,
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
        activate_addresses: Mutex::new((init_activate_num, vec![true; init_activate_num])),
        ratio_limit: Mutex::new(HashMap::new()),
    });

    log::info!("ProxyState {:?}", state);

    { // activate health check
        let state = state.clone();
        tokio::spawn(async move {
            active_health_check(state).await;
        });
    }

    if state.max_requests_per_minute != 0 { // Rate limiting
        let state = state.clone();
        tokio::spawn(async move {
            rate_limiting_refresh(state, SECONDS_PER_MINUTE).await;
        });
    }

    loop {
        let (socket, _) = listener.accept().await?;
        handle_connection(socket, &state).await;
    }
}

async fn connect_to_upstream(state: &ProxyState) -> Result<TcpStream, std::io::Error> {
    let mut rng = rand::rngs::StdRng::from_entropy();
    loop {
        let upstream_idx;
        { // Reduce the granularity of the lock
            let active_addrs = state.activate_addresses.lock().await;
            if active_addrs.0 == 0 {
                return Err(std::io::Error::new(ErrorKind::Other, "All the upstream servers are down!"));
            }
            upstream_idx = rng.gen_range(0, state.upstream_addresses.len());
            if !active_addrs.1[upstream_idx] {
                continue;
            }
        }
        let upstream_ip = &state.upstream_addresses[upstream_idx];
        if let Ok(stream) = TcpStream::connect(upstream_ip).await {
            return Ok(stream);
        } else {
            { // Reduce the granularity of the lock
                let mut active_addrs = state.activate_addresses.lock().await;
                if active_addrs.1[upstream_idx] { // double check
                    active_addrs.0 -= 1;
                    active_addrs.1[upstream_idx] = false;
                }
            }
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("{} <- {}", client_ip, response::format_response_line(&response));
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: &ProxyState) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);
    if state.max_requests_per_minute != 0 {
        let mut ratio_limit_map = state.ratio_limit.lock().await;
        if !ratio_limit_map.contains_key(&client_ip) {
            ratio_limit_map.insert(client_ip.clone(), 0);
        }
        let new_cnt = *ratio_limit_map.get(&client_ip).unwrap() + 1;
        ratio_limit_map.insert(client_ip.clone(), new_cnt);
        log::warn!("[ratio limit] ip: {}, count {}", client_ip, new_cnt);
        if new_cnt > state.max_requests_per_minute {
            let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
            send_response(&mut client_conn, &response).await;
            return;
        }
    }

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(&state).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = upstream_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!("Failed to send request to upstream {}: {}", upstream_ip, error);
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}

async fn check_server(ip_idx: usize, state: &ProxyState) -> bool {
    let ip_addr = &state.upstream_addresses[ip_idx];
    if let Ok(mut stream) = TcpStream::connect(ip_addr).await {
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(&state.active_health_check_path)
            .header("Host", ip_addr)
            .body(Vec::new())
            .unwrap();
        if request::write_to_stream(&request, &mut stream).await.is_ok() {
            if let Ok(resp) = response::read_from_stream(&mut stream, &http::Method::GET).await {
                if resp.status().as_u16() == 200 {
                    return true;
                }
            }
        }
    }
    false
}

async fn active_health_check(state: Arc<ProxyState>) {
    let interval = state.active_health_check_interval as u64;
    loop {
        delay_for(Duration::from_secs(interval)).await;
        for ip_idx in 0..state.upstream_addresses.len() {
            let mut active_addrs = state.activate_addresses.lock().await;
            if check_server(ip_idx, &state).await {
                if active_addrs.1[ip_idx] == false {
                    active_addrs.0 += 1;
                    active_addrs.1[ip_idx] = true;
                }
            } else {
                if active_addrs.1[ip_idx] == true {
                    active_addrs.0 -= 1;
                    active_addrs.1[ip_idx] = false;
                }
            }
        }
    }
}

async fn rate_limiting_refresh(state: Arc<ProxyState>, refresh_interval: u64) {
    loop {
        delay_for(Duration::from_secs(refresh_interval)).await;
        state.ratio_limit.lock().await.clear();
    }
}

