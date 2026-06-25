use serde::Deserialize;

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
