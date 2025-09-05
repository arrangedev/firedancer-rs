use core::net::Ipv4Addr;
use fd_http::{
    ConnectionCloseReason, Method, Request, Response, Server, ServerCallbacks, ServerParams,
};

struct API {
    name: String,
}

impl API {
    fn new(name: String) -> Self {
        Self { name }
    }

    fn handle_get(&self, path: &str) -> Response {
        match path {
            "/" => Response::ok()
                .header("content-type", "text/html")
                .text(&format!(
                    r#"<!DOCTYPE html>
<html>
<head><title>{} Server</title></head>
<body>
    <h1>Welcome to {}!</h1>
    <p>Available endpoints:</p>
    <ul>
        <li><a href="/api/status">GET /api/status</a> - Server status</li>
        <li><a href="/api/info">GET /api/info</a> - Server information</li>
        <li>POST /api/echo - Echo request body</li>
    </ul>
</body>
</html>"#,
                    self.name, self.name
                )),

            "/api/status" => Response::ok()
                .header("content-type", "application/json")
                .header("cache-control", "no-cache")
                .text(r#"{"status":"healthy","uptime":"unknown"}"#),

            "/api/info" => Response::ok()
                .header("content-type", "application/json")
                .header("access-control-allow-origin", "*")
                .text(&format!(
                    r#"{{"server":"{}","version":"1.0.0","features":["http","websockets"]}}"#,
                    self.name
                )),

            "/favicon.ico" => Response::new(404).text("Not Found"),

            _ => Response::new(404)
                .header("content-type", "text/plain")
                .text("404 Not Found"),
        }
    }

    fn handle_post(&self, path: &str, body: &[u8]) -> Response {
        match path {
            "/api/echo" => {
                let body_text = core::str::from_utf8(body).unwrap_or("<invalid utf8>");
                Response::ok()
                    .header("content-type", "application/json")
                    .text(&format!(
                        r#"{{"echo":"{}","length":{}}}"#,
                        body_text,
                        body.len()
                    ))
            }
            _ => Response::new(405)
                .header("allow", "GET")
                .text("Method Not Allowed"),
        }
    }
}

impl ServerCallbacks for API {
    fn on_request(&mut self, conn_id: u64, request: Request) -> Response {
        let method_str = match request.method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Options => "OPTIONS",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Patch => "PATCH",
        };

        println!("[{conn_id}] {method_str} {}", request.path);

        if request.method == Method::Options {
            return Response::ok()
                .header("access-control-allow-origin", "*")
                .header("access-control-allow-methods", "GET, POST, OPTIONS")
                .header("access-control-allow-headers", "content-type")
                .header("access-control-max-age", "86400");
        }

        match request.method {
            Method::Get => self.handle_get(request.path),
            Method::Post => self.handle_post(request.path, request.body),
            Method::Put | Method::Delete | Method::Head | Method::Patch => Response::new(405)
                .header("allow", "GET, POST, OPTIONS")
                .text("Method Not Allowed"),
            Method::Options => unreachable!(),
        }
    }

    fn on_connection_close(&mut self, conn_id: u64, reason: ConnectionCloseReason) {
        // cleanup any per-connection state
        let reason_str = match reason {
            ConnectionCloseReason::Ok => "normal",
            ConnectionCloseReason::PeerClose => "peer_closed",
            ConnectionCloseReason::Error => "error",
            ConnectionCloseReason::Timeout => "timeout",
        };
        println!("{conn_id} closed: {reason_str}");
    }

    fn on_ws_connect(&mut self, connection_id: u64, path: &str) -> bool {
        println!("WebSocket request to {path} from {connection_id}");
        true // accept all WS connections
    }

    fn on_ws_message(&mut self, connection_id: u64, message: &[u8]) {
        println!("WS {connection_id} received {} bytes", message.len());
        let _ = (connection_id, message);
    }

    // WS close is handled by on_connection_close
}

fn main() -> Result<(), fd_http::Error> {
    let params = ServerParams::builder()
        .max_request_len(8192)        // 8KB max req size
        .max_ws_recv_frame_len(8192)  // 8KB max ws frame
        .build();

    let callbacks = API::new("libfd-example".to_string());
    let mut server = Server::new(params, callbacks, 65536)?;

    let addr = Ipv4Addr::new(127, 0, 0, 1);
    server.listen(addr, 8080)?;

    println!("√ Server listening on http://{addr}:8080");
    println!();
    println!("Endpoints:");
    println!("   curl http://localhost:8080/");
    println!("   curl http://localhost:8080/api/status");
    println!("   curl http://localhost:8080/api/info");
    println!("   curl -X POST http://localhost:8080/api/echo -d 'yo!'");
    println!();
    println!("Press Ctrl+C to stop");

    loop {
        match server.poll() {
            Ok(_) => {
                if server.connection_count() > 0 {
                    println!("Connections: {}", server.connection_count());
                }
            }
            Err(e) => {
                eprintln!("x Server error: {e}");
                let _ = e;
                break;
            }
        }
    }

    println!("Server shutting down...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_server_get_endpoints() {
        let mut server = API::new("Test".to_string());

        let request = Request {
            method: Method::Get,
            path: "/",
            headers: Vec::new(),
            body: &[],
        };
        let response = server.on_request(1, request);
        assert_eq!(response.status(), 200);
        assert!(response.body_bytes().len() > 0);

        // Test status endpoint
        let request = Request {
            method: Method::Get,
            path: "/api/status",
            headers: Vec::new(),
            body: &[],
        };
        let response = server.on_request(1, request);
        assert_eq!(response.status(), 200);
        assert!(core::str::from_utf8(response.body_bytes())
            .unwrap()
            .contains("healthy"));

        // Test 404
        let request = Request {
            method: Method::Get,
            path: "/nonexistent",
            headers: Vec::new(),
            body: &[],
        };
        let response = server.on_request(1, request);
        assert_eq!(response.status(), 404);
    }

    #[test]
    fn test_api_server_post_echo() {
        let mut server = API::new("Test".to_string());

        let request = Request {
            method: Method::Post,
            path: "/api/echo",
            headers: Vec::new(),
            body: b"Hello, World!",
        };
        let response = server.on_request(1, request);
        assert_eq!(response.status(), 200);

        let body_str = core::str::from_utf8(response.body_bytes()).unwrap();
        assert!(body_str.contains("Hello, World!"));
        assert!(body_str.contains("13")); // length
    }

    #[test]
    fn test_cors_preflight() {
        let mut server = API::new("Test".to_string());

        let request = Request {
            method: Method::Options,
            path: "/api/status",
            headers: Vec::new(),
            body: &[],
        };
        let response = server.on_request(1, request);
        assert_eq!(response.status(), 200);

        // Check CORS headers are present
        let has_cors_origin = response
            .response_headers()
            .iter()
            .any(|(key, value)| key == "access-control-allow-origin" && value == "*");
        assert!(has_cors_origin);
    }
}
