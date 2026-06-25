import "./styles.css";

export default function App() {
  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>AlphaPulse OKX</h1>
          <p>USDT perpetual radar</p>
        </div>
        <dl className="status-grid" aria-label="connection status">
          <div>
            <dt>Backend</dt>
            <dd>Disconnected</dd>
          </div>
          <div>
            <dt>Stream</dt>
            <dd>Idle</dd>
          </div>
          <div>
            <dt>Notifications</dt>
            <dd>Not requested</dd>
          </div>
        </dl>
      </header>
      <section className="empty-state">
        <h2>No symbols loaded</h2>
        <p>Start the Rust backend to populate the radar.</p>
      </section>
    </main>
  );
}
