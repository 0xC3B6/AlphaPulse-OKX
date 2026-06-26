use alphapulse_okx_backend::okx::{
    rest::{parse_candles, parse_instruments, parse_tickers},
    ws::parse_ticker_event,
};

#[test]
fn parses_okx_candle_arrays() {
    let json = r#"{
        "code":"0",
        "msg":"",
        "data":[["1782387000000","16.938","17.241","16.936","17.182","9847.3","98473","1685633.336","1"]]
    }"#;

    let candles = parse_candles(json).unwrap();

    assert_eq!(candles.len(), 1);
    assert_eq!(candles[0].ts_ms, 1782387000000);
    assert_eq!(candles[0].open, 16.938);
    assert_eq!(candles[0].close, 17.182);
    assert_eq!(candles[0].volume, 9847.3);
}

#[test]
fn parses_okx_ticker_rows() {
    let json = r#"{
        "code":"0",
        "msg":"",
        "data":[{"instId":"LAB-USDT-SWAP","last":"17.187","volCcy24h":"20113997","ts":"1782387679663"}]
    }"#;

    let tickers = parse_tickers(json).unwrap();

    assert_eq!(tickers.len(), 1);
    assert_eq!(tickers[0].inst_id, "LAB-USDT-SWAP");
    assert_eq!(tickers[0].last, 17.187);
    assert_eq!(tickers[0].quote_volume_24h, 20113997.0);
}

#[test]
fn parses_okx_instrument_rows() {
    let json = r#"{
        "code":"0",
        "msg":"",
        "data":[{"instId":"LAB-USDT-SWAP","state":"live","listTime":"1781800000000"}]
    }"#;

    let instruments = parse_instruments(json).unwrap();

    assert_eq!(instruments.len(), 1);
    assert_eq!(instruments[0].inst_id, "LAB-USDT-SWAP");
    assert_eq!(instruments[0].state, "live");
    assert_eq!(instruments[0].list_time_ms, 1781800000000);
}

#[test]
fn parses_okx_ws_ticker_event() {
    let json = r#"{
        "arg":{"channel":"tickers","instId":"LAB-USDT-SWAP"},
        "data":[{"instId":"LAB-USDT-SWAP","last":"17.187","ts":"1782387679663"}]
    }"#;

    let event = parse_ticker_event(json).unwrap().unwrap();

    assert_eq!(event.inst_id, "LAB-USDT-SWAP");
    assert_eq!(event.last, 17.187);
    assert_eq!(event.ts_ms, 1782387679663);
}
