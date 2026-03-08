// src/build/reload.rs

//! Live reload functionality for development mode.
//!
//! This module provides WebSocket-based live reload for the development server.
//! When a successful frontend build completes, all connected browser tabs
//! automatically refresh to show the changes.
//!
//! # Components
//!
//! - [`RELOAD_SCRIPT`] — JavaScript that establishes a WebSocket connection
//! - [`inject_reload_script`] — Inserts the script into HTML
//! - [`ws_reload_handler`] — WebSocket endpoint handler

use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::stream::StreamExt;
use tokio::sync::broadcast;

use super::DevServerMessage;

/// JavaScript that establishes a WebSocket connection to /ws/reload
/// and reloads the page when a message is received.
///
/// # Key Features
///
/// - Automatic reconnection on close (1 second delay)
/// - Uses `location.host` to work with any port
/// - No explicit `onerror` handler needed — errors trigger `onclose` anyway
///
/// # Injection
///
/// This script is injected immediately before the `</body>` tag in dev mode.
/// It is NOT present in the `index.html` file on disk.
pub const RELOAD_SCRIPT: &str = r#"<script>
(function() {
  function removeOverlay() {
    var existing = document.getElementById('__drydock_error__');
    if (existing) existing.remove();
  }

  function showOverlay(message) {
    removeOverlay();
    var overlay = document.createElement('div');
    overlay.id = '__drydock_error__';
    overlay.style.cssText = [
      'position: fixed',
      'top: 0',
      'left: 0',
      'width: 100%',
      'height: 100%',
      'background: rgba(0,0,0,0.85)',
      'color: #ff5555',
      'font-family: monospace',
      'font-size: 14px',
      'padding: 2rem',
      'box-sizing: border-box',
      'z-index: 2147483647',
      'white-space: pre-wrap',
      'overflow: auto',
    ].join(';');
    var heading = document.createElement('div');
    heading.style.cssText = 'font-size: 1.2rem; margin-bottom: 1rem; color: #ff5555;';
    heading.textContent = 'Build Failed';
    var body = document.createElement('div');
    body.textContent = message;
    overlay.appendChild(heading);
    overlay.appendChild(body);
    document.body.appendChild(overlay);
  }

  function connect() {
    var ws = new WebSocket('ws://' + location.host + '/ws/reload');
    ws.onmessage = function(event) {
      if (event.data === 'reload') {
        removeOverlay();
        location.reload();
      } else if (event.data.startsWith('error:')) {
        showOverlay(event.data.slice(6));
      }
    };
    ws.onclose = function() { setTimeout(connect, 1000); };
  }
  connect();
})();
</script>"#;

/// Inject the reload script into HTML content.
///
/// Inserts [`RELOAD_SCRIPT`] immediately before the last `</body>` tag.
/// If `</body>` is not found, appends the script at the end.
///
/// # Arguments
///
/// * `html` — The HTML content to inject into
///
/// # Returns
///
/// A new String with the reload script injected.
///
/// # Examples
///
/// ```
/// use wasm_drydock::inject_reload_script;
///
/// let html = "<html><body><p>Hello</p></body></html>";
/// let result = inject_reload_script(html);
/// assert!(result.contains("/ws/reload"));
/// assert!(result.find("/ws/reload").unwrap() < result.find("</body>").unwrap());
/// ```
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
///
/// Upgrades the HTTP connection to WebSocket and subscribes to
/// the broadcast channel. When a reload signal is received,
/// sends "reload" text frame to the browser.
///
/// # Arguments
///
/// * `req` — The HTTP request (used for WebSocket upgrade and peer address)
/// * `body` — The request payload (required by actix-ws)
/// * `reload_tx` — Broadcast sender wrapped in `web::Data`
///
/// # Behavior
///
/// 1. Upgrade connection to WebSocket via `actix_ws::handle`
/// 2. Subscribe to the broadcast channel
/// 3. Spawn a task that:
///    - Listens for reload signals on the broadcast channel
///    - Sends "reload" text frame on signal
///    - Handles `Lagged` error by sending one reload
///    - Handles client disconnect (close frame or stream end)
///    - Handles broadcast channel closure
///
/// # Error Handling
///
/// - `RecvError::Lagged` — Logs warning, sends one reload signal
/// - `RecvError::Closed` — Logs debug, exits loop
/// - WebSocket send failure — Logs debug, exits loop (client disconnected)
///
/// # Example Route Registration
///
/// ```ignore
/// App::new()
///     .app_data(web::Data::new(reload_tx))
///     .route("/ws/reload", web::get().to(ws_reload_handler))
/// ```
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<DevServerMessage>>,
) -> Result<HttpResponse, actix_web::Error> {
    let peer = req
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".into());

    let span = tracing::debug_span!("ws_connection", peer = %peer);
    let _enter = span.enter();

    // Origin validation for CSRF protection
    // Reject cross-origin WebSocket connections to prevent
    // malicious websites from connecting to the dev server
    if let Some(origin) = req.headers().get("origin") {
        // Get host as an owned String to avoid lifetime issues
        let host = req.connection_info().host().to_string();
        // Allow connections from the same host (development scenario)
        let expected_origin = format!("http://{}", host);
        let expected_origin_https = format!("https://{}", host);

        // Convert HeaderValue to string for comparison
        let origin_str = origin.to_str().unwrap_or("");

        if origin_str != expected_origin && origin_str != expected_origin_https {
            tracing::warn!(
                origin = %origin_str,
                expected = %expected_origin,
                "WebSocket connection rejected - origin mismatch"
            );
            return Err(actix_web::error::ErrorForbidden("Invalid origin"));
        }
    }

    tracing::debug!("WebSocket connection established");

    let mut reload_rx = reload_tx.subscribe();
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                // Reload signal from build coordinator
                result = reload_rx.recv() => {
                    match result {
                        Ok(DevServerMessage::Reload) => {
                            tracing::debug!("sending reload signal to browser");
                            if session.text("reload").await.is_err() {
                                tracing::debug!("WebSocket send failed — client disconnected");
                                break;
                            }
                        }
                        Ok(DevServerMessage::BuildError(msg)) => {
                            tracing::debug!("sending build error to browser");
                            let payload = format!("error:{}", msg);
                            if session.text(payload).await.is_err() {
                                tracing::debug!("WebSocket send failed — client disconnected");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Subscriber fell behind — this means multiple rebuilds happened
                            // faster than this connection was drained. Reload once now.
                            tracing::warn!(skipped = n, "WebSocket subscriber lagged — sending one reload");
                            let _ = session.text("reload").await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("broadcast channel closed — closing WebSocket");
                            break;
                        }
                    }
                }
                // Client disconnected or sent a close frame
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
                        _ => {} // Ping/Pong/Text from client — ignore
                    }
                }
            }
        }
    });

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_injected_before_body_close_tag() {
        let html = "<html><body><p>Hello</p></body></html>";
        let result = inject_reload_script(html);
        let script_pos = result.find(RELOAD_SCRIPT).expect("script not found");
        let body_pos = result.find("</body>").expect("</body> not found");
        assert!(
            script_pos < body_pos,
            "script should be injected before </body>, but script is at {} and </body> is at {}",
            script_pos,
            body_pos
        );
    }

    #[test]
    fn handles_html_without_body_tag_gracefully() {
        let html = "<html><p>No body tag</p></html>";
        let result = inject_reload_script(html);
        assert!(
            result.contains(RELOAD_SCRIPT),
            "script should be appended when </body> is absent"
        );
    }

    #[test]
    fn reload_script_references_ws_reload_path() {
        assert!(
            RELOAD_SCRIPT.contains("/ws/reload"),
            "reload script must reference /ws/reload WebSocket path"
        );
    }

    #[test]
    fn reload_script_contains_onclose_reconnect() {
        assert!(
            RELOAD_SCRIPT.contains("onclose"),
            "reload script must include reconnect logic — see Section 4.3 of spec"
        );
    }

    #[test]
    fn inject_reload_script_preserves_original_content() {
        let html = "<html><body><p>Hello World</p></body></html>";
        let result = inject_reload_script(html);
        assert!(
            result.contains("<p>Hello World</p>"),
            "original content should be preserved"
        );
    }

    #[test]
    fn inject_reload_script_with_multiple_body_tags() {
        // Edge case: multiple </body> tags (malformed HTML)
        let html = "<html><body></body></body></html>";
        let result = inject_reload_script(html);
        // Should inject before the LAST </body>
        let script_pos = result.find(RELOAD_SCRIPT).unwrap();
        // The script should be before the last </body> in the result
        assert!(
            script_pos < result.rfind("</body>").unwrap(),
            "script should be before the last </body>"
        );
    }

    #[test]
    fn broadcast_send_with_no_receivers_does_not_panic() {
        let (tx, _rx) = broadcast::channel::<DevServerMessage>(16);
        drop(_rx);
        let result = tx.send(DevServerMessage::Reload);
        // The send returns Err when there are no receivers
        // This is expected and acceptable behavior
        assert!(result.is_err(), "send should return Err when no receivers");
    }

    #[test]
    fn broadcast_send_with_receivers_succeeds() {
        let (tx, _rx) = broadcast::channel::<DevServerMessage>(16);
        let result = tx.send(DevServerMessage::Reload);
        assert!(result.is_ok(), "send should succeed when receivers exist");
    }
}
