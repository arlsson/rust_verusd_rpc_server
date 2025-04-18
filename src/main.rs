use hyper::{Body, Request, Response, Server, service::{make_service_fn, service_fn}};
use serde_json::{Value, json};
use jsonrpc::{Client, error::RpcError};
use jsonrpc::simple_http::{self, SimpleHttpTransport};
use serde_json::value::RawValue;
use std::sync::{Arc, Mutex};
use std::env;
use std::process::exit;

mod conf;
mod allowlist;

struct VerusRPC {
    client: Arc<Mutex<Client>>,
}

impl VerusRPC {
    fn new(url: &str, user: &str, pass: &str) -> Result<VerusRPC, simple_http::Error> {
        let transport = SimpleHttpTransport::builder()
            .url(url)?
            .auth(user, Some(pass))
            .build();
        Ok(VerusRPC { client: Arc::new(Mutex::new(Client::with_transport(transport))) })
    }

    fn handle(&self, req_body: Value) -> Result<Value, RpcError> {
        let method = match req_body["method"].as_str() {
            Some(method) => method,
            None => return Err(RpcError { code: -32602, message: "Invalid method parameter".into(), data: None }),
        };
        let params: Vec<Box<RawValue>> = match req_body["params"].as_array() {
            Some(params) => {
                params.iter().enumerate().map(|(i, v)| {
                    if method == "getblock" && i == 0 {
                        if let Ok(num) = v.to_string().parse::<i64>() {
                            // Legacy hack because getblock in JS used to allow 
                            // strings to be passed in clientside and the former JS rpc server
                            // wouldn't care. This will be deprecated in the future and shouldn't
                            // be relied upon.
                            RawValue::from_string(format!("\"{}\"", num)).unwrap()
                        } else {
                            RawValue::from_string(v.to_string()).unwrap()
                        }
                    } else {
                        RawValue::from_string(v.to_string()).unwrap()
                    }
                }).collect()
            },
            None => return Err(RpcError { code: -32602, message: "Invalid params parameter".into(), data: None }),
        };
    
        if !allowlist::is_method_allowed(method, &params) {
            return Err(RpcError { code: -32601, message: "Method not found".into(), data: None });
        }
    
        let client = self.client.lock().unwrap();
        let request = client.build_request(method, &params);

        let response = client.send_request(request).map_err(|e| match e {
            jsonrpc::Error::Rpc(rpc_error) => rpc_error,
            _ => RpcError { code: -32603, message: "Internal error".into(), data: None },
        })?;
        
        let result: Value = response.result().map_err(|e| match e {
            jsonrpc::Error::Rpc(rpc_error) => rpc_error,
            _ => RpcError { code: -32603, message: "Internal error".into(), data: None },
        })?;
        Ok(result)
    }
}

async fn handle_req(req: Request<Body>, rpc: Arc<VerusRPC>) -> Result<Response<Body>, hyper::Error> {

    // Handle CORS preflight (OPTIONS) request
    if req.method() == hyper::Method::OPTIONS {
        let mut response = Response::new(Body::empty());
        response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST".parse().unwrap());
        response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization, Accept".parse().unwrap());
        response.headers_mut().insert(hyper::header::ACCESS_CONTROL_MAX_AGE, "3600".parse().unwrap());
        return Ok(response);
    }

    // Maximum allowed content length (in bytes)
    const MAX_CONTENT_LENGTH: u64 = 1024 * 1024 * 50; // 50 MiB, adjust as needed

    if let Some(content_length) = req.headers().get(hyper::header::CONTENT_LENGTH) {
        if let Ok(content_length) = content_length.to_str().unwrap_or("").parse::<u64>() {
            if content_length > MAX_CONTENT_LENGTH {
                return Ok(Response::builder()
                    .status(hyper::StatusCode::PAYLOAD_TOO_LARGE)
                    .body(Body::from("Payload too large"))
                    .unwrap());
            }
        }
    }
    
    let whole_body = hyper::body::to_bytes(req.into_body()).await?;
    let str_body = String::from_utf8(whole_body.to_vec()).unwrap();
    let json_body: Result<Value, _> = serde_json::from_str(&str_body);
    let result = match json_body {
        Ok(req_body) => rpc.handle(req_body),
        Err(_) => Err(RpcError { code: -32700, message: "Parse error".into(), data: None }),
    };
    // Process the CORS headers
    let mut response = match result {
        Ok(res) => Response::new(Body::from(json!({"result": res}).to_string())),
        Err(err) => Response::new(Body::from(json!({"error": { "code": err.code, "message": err.message }}).to_string())),
    };

    // Add CORS headers
    response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, HEAD, PUT, OPTIONS, POST".parse().unwrap());
    response.headers_mut().insert(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization, Accept".parse().unwrap());
    response.headers_mut().insert(hyper::header::ACCESS_CONTROL_MAX_AGE, "3600".parse().unwrap());

    // Set the Referrer Policy header
    response.headers_mut().insert(hyper::header::REFERRER_POLICY, "origin-when-cross-origin".parse().unwrap());

    Ok(response)

}

#[tokio::main]
async fn main() {

    if env::args().nth(1).as_deref() == Some("--version") {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");
        println!("{name} {version}");
        exit(0);
    }

    let (rpc_url, rpc_user, rpc_password, server_addr, server_port) = conf::load_settings("Conf");

    let addr = (server_addr.parse::<std::net::IpAddr>().unwrap(), server_port).into();

    let make_svc = make_service_fn(|_conn| {
        let rpc = Arc::new(VerusRPC::new(&rpc_url, &rpc_user, &rpc_password).unwrap());
        async {
            Ok::<_, hyper::Error>(service_fn(move |req| handle_req(req, rpc.clone())))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running at {}:{}", server_addr, server_port);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

}
