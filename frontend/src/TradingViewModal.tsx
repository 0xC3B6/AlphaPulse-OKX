import type { Copy, Language } from "./i18n";
import type { SymbolSnapshot } from "./types";
import type { ThemeMode } from "./uiFormat";

export function TradingViewModal({
  copy,
  language,
  onClose,
  symbol,
  themeMode,
}: {
  copy: Copy;
  language: Language;
  onClose: () => void;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
}) {
  const tradingViewSymbol = resolveTradingViewSymbol(symbol.inst_id);
  const title = `${symbol.inst_id} TradingView`;
  return (
    <div className="tv-modal-backdrop" onClick={onClose}>
      <section
        aria-label={title}
        aria-modal="true"
        className="tv-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header>
          <div>
            <h2>{symbol.inst_id}</h2>
            <p>{tradingViewSymbol ?? copy.detail.tradingViewUnavailable}</p>
          </div>
          <button
            aria-label={copy.actions.closeTradingView}
            className="tv-modal-close"
            onClick={onClose}
            type="button"
          >
            x
          </button>
        </header>
        <div className="tv-modal-frame-wrap">
          {tradingViewSymbol ? (
            <iframe
              allow="fullscreen"
              src={buildTradingViewEmbedUrl(tradingViewSymbol, themeMode, language)}
              title={title}
            />
          ) : (
            <div className="tv-modal-empty">{copy.detail.tradingViewUnavailable}</div>
          )}
        </div>
      </section>
    </div>
  );
}

function resolveTradingViewSymbol(instId: string): string | null {
  const normalized = instId.toUpperCase().replace(/[^A-Z0-9-]/g, "");
  const swapMatch = normalized.match(/^([A-Z0-9]+)-USDT-SWAP$/);
  if (swapMatch) {
    return `OKX:${swapMatch[1]}USDT.P`;
  }

  const spotMatch = normalized.match(/^([A-Z0-9]+)-USDT$/);
  if (spotMatch) {
    return `OKX:${spotMatch[1]}USDT`;
  }

  const compact = normalized.replace(/-/g, "");
  return compact.length > 0 ? `OKX:${compact}` : null;
}

function buildTradingViewEmbedUrl(symbol: string, themeMode: ThemeMode, language: Language): string {
  const locale = language === "zh" ? "zh_CN" : language === "ja" ? "ja" : "en";
  const params = new URLSearchParams({
    symbol,
    interval: "15",
    theme: themeMode === "light" ? "light" : "dark",
    style: "1",
    locale,
    enable_publishing: "0",
    allow_symbol_change: "0",
    hide_top_toolbar: "0",
    withdateranges: "1",
  });
  return `https://s.tradingview.com/widgetembed/?${params.toString()}`;
}
