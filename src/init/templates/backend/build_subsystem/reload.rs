use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::stream::StreamExt;
use tokio::sync::broadcast;

/// JavaScript that establishes a WebSocket connection for live reload.
pub const RELOAD_SCRIPT: &str = r#"<script>
(function() {
  function connect() {
    const ws = new WebSocket('ws://' + location.host + '/ws/reload');
    ws.onmessage = () => location.reload();
    ws.onclose = () => setTimeout(connect, 1000);
  }
  connect();
})();
</script>"#;

/// Inject the reload script into HTML content.
pub fn inject_reload_script(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(&html[..pos]);
        result.push_str(RELOAD_SCRIPT);
        result.push_str(&html[pos..]);
        result
    } else {
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(html);
        result.push_str(RELOAD_SCRIPT);
        result
    }
}

/// WebSocket handler for live reload signaling.
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<()>>,
) -> Result<HttpResponse, actix_web::Error> {
    let peer = req.peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".into());

    let span = tracing::debug_span!("ws_connection", peer = %peer);
    let _enter = span.enter();
    tracing::debug!("WebSocket connection established");

    let mut reload_rx = reload_tx.subscribe();
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                result = reload_rx.recv() => {
                    match result {
                        Ok(()) => {
                            tracing::debug!("sending reload signal to browser");
                            if session.text("reload").await.is_err() {
                                tracing::debug!("WebSocket send failed — client disconnected");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "WebSocket subscriber lagged — sending one reload");
                            let _ = session.text("reload").await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("broadcast channel closed — closing WebSocket");
                            break;
                        }
                    }
                }
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(Message::Close(reason))) => {
                            tracing::debug!(?reason, "WebSocket client closed connection");
                            let _ = session.close(reason).await;
                            break;
                        }
                        None | Some(Err(_)) => {
                            tracing::debug!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    Ok(response)
}