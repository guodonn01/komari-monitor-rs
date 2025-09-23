use crate::callbacks::ping::ping_target;
use crate::command_parser::Args;
use crate::utils::ConnectionUrls;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{error, info};
use miniserde::{Deserialize, Serialize, json};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub mod ping;

#[derive(Serialize, Deserialize)]
struct Msg {
    message: String,
}

pub async fn handle_callbacks(
    _args: &Args,
    _connection_urls: &ConnectionUrls,
    reader: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    locked_writer: &Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
) -> () {
    while let Some(msg) = reader.next().await {
        let Ok(msg) = msg else {
            continue;
        };

        let Ok(utf8) = msg.into_text() else {
            continue;
        };

        info!("主端传入信息: {}", utf8.as_str());

        let json: Msg = if let Ok(value) = json::from_str(utf8.as_str()) {
            value
        } else {
            continue;
        };

        let utf8_cloned = utf8.clone();

        match json.message.as_str() {
            "ping" => {
                let locked_write_for_ping = locked_writer.clone();
                tokio::spawn(async move {
                    match ping_target(&utf8_cloned).await {
                        Ok(json_res) => {
                            let mut write = locked_write_for_ping.lock().await;
                            info!("Ping Success: {}", json::to_string(&json_res));
                            if let Err(e) = write
                                .send(Message::Text(Utf8Bytes::from(json::to_string(&json_res))))
                                .await
                            {
                                error!("推送 ping result 时发生错误，尝试重新连接: {e}");
                            }
                        }
                        Err(err) => {
                            error!("Ping Error: {err}");
                        }
                    }
                });
            }
            _ => {}
        }
    }
}
