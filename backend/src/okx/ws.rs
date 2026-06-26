use serde::Deserialize;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, PartialEq)]
pub struct TickerEvent {
    pub inst_id: String,
    pub last: f64,
    pub ts_ms: i64,
}

#[derive(Debug, Deserialize)]
struct WsEnvelope {
    data: Option<Vec<RawWsTicker>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWsTicker {
    inst_id: String,
    last: String,
    ts: String,
}

pub fn parse_ticker_event(json: &str) -> anyhow::Result<Option<TickerEvent>> {
    let envelope: WsEnvelope = serde_json::from_str(json)?;
    let Some(mut data) = envelope.data else {
        return Ok(None);
    };
    let Some(row) = data.pop() else {
        return Ok(None);
    };
    Ok(Some(TickerEvent {
        inst_id: row.inst_id,
        last: row.last.parse()?,
        ts_ms: row.ts.parse()?,
    }))
}

pub async fn stream_tickers(
    inst_ids: Vec<String>,
    sender: mpsc::Sender<TickerEvent>,
) -> anyhow::Result<()> {
    if inst_ids.is_empty() {
        return Ok(());
    }

    let (socket, _) = connect_async("wss://ws.okx.com:8443/ws/v5/public").await?;
    let (mut write, mut read) = socket.split();
    let args: Vec<_> = inst_ids
        .iter()
        .map(|inst_id| json!({ "channel": "tickers", "instId": inst_id }))
        .collect();
    write
        .send(Message::Text(
            json!({ "op": "subscribe", "args": args }).to_string(),
        ))
        .await?;

    while let Some(message) = read.next().await {
        match message? {
            Message::Text(text) => {
                if let Some(event) = parse_ticker_event(&text)? {
                    sender.send(event).await?;
                }
            }
            Message::Ping(payload) => {
                write.send(Message::Pong(payload)).await?;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}
