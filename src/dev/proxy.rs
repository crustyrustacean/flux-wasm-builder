// src/dev/proxy.rs

// dependencies
use crate::domain::DrydockConfig;
use actix_web::{HttpRequest, HttpResponse, web};
use reqwest::Client;

pub async fn proxy_handler(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
    config: web::Data<DrydockConfig>,
) -> HttpResponse {
    let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("");
    let target_url = format!("http://localhost:{}{}", config.dev.backend_port, path);

    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut forwarded = client.request(method, &target_url).body(body);

    // Forward headers, skipping ones that would cause problems
    for (name, value) in req.headers() {
        if name != "host"
            && name != "transfer-encoding"
            && let Ok(v) = value.to_str()
        {
            forwarded = forwarded.header(name.as_str(), v);
        }
    }

    match forwarded.send().await {
        Ok(response) => {
            let status = response.status();
            let mut builder = HttpResponse::build(
                actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap(),
            );
            for (name, value) in response.headers() {
                if name != "transfer-encoding"
                    && let Ok(v) = value.to_str()
                {
                    builder.insert_header((name.as_str(), v));
                }
            }
            let bytes = response.bytes().await.unwrap_or_default();
            builder.body(bytes)
        }
        Err(e) => {
            eprintln!("Proxy error: {e}");
            HttpResponse::BadGateway().finish()
        }
    }
}
