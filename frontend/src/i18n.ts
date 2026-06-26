export type Language = "zh" | "en";

export const defaultLanguage: Language = "zh";

export const translations = {
  zh: {
    subtitle: "USDT 永续雷达",
    aria: {
      connectionStatus: "连接状态",
      radarControls: "雷达控制",
      opportunityFilters: "机会筛选",
      themeMode: "主题模式",
      languageMode: "语言",
    },
    status: {
      backend: "后端",
      stream: "行情流",
      notifications: "通知",
      lastScan: "最近扫描",
      symbols: "合约数",
    },
    states: {
      connected: "已连接",
      disconnected: "未连接",
      idle: "空闲",
      unsupported: "不支持",
      granted: "已允许",
      denied: "已拒绝",
      default: "未设置",
    },
    filters: {
      all: "全部",
      trend: "趋势",
      range: "震荡",
      hot: "热门",
      fixed: "固定",
    },
    themes: {
      light: "浅色主题",
      dark: "深色主题",
      system: "跟随系统",
    },
    languages: {
      zh: "中文",
      en: "English",
    },
    actions: {
      enableNotifications: "开启通知",
    },
    empty: {
      title: "暂无合约数据",
      body: "启动 Rust 后端后会填充雷达数据。",
    },
    table: {
      symbol: "合约",
      price: "价格",
      trend: "趋势",
      range: "震荡",
      signal: "信号",
    },
    detail: {
      noActiveTrigger: "暂无触发信号",
      funding: "资金费率",
      updated: "更新时间",
      fvg: "FVG",
      noFvgZones: "暂无 FVG 区域",
      levels: "关键位",
      noLevels: "暂无关键位",
      distance: "距离",
      touches: "触达",
      account: "账户",
      noApiKey: "未连接只读 OKX API Key。",
    },
    misc: {
      unlabeled: "未标记",
      watching: "观察中",
    },
    directions: {
      long: "多",
      short: "空",
      neutral: "中性",
    },
    poolTags: {
      dynamic: "动态",
      fixed: "固定",
      manual_watch: "手动关注",
      new_listing: "新币",
      thin_history: "历史较短",
      low_market_cap: "低市值",
      volume_mcap_anomaly: "量市值异常",
      derivative_dominated: "合约拥挤",
      identity_uncertain: "身份待确认",
    },
    levelKinds: {
      support: "支撑",
      resistance: "压力",
    },
  },
  en: {
    subtitle: "USDT perpetual radar",
    aria: {
      connectionStatus: "connection status",
      radarControls: "radar controls",
      opportunityFilters: "opportunity filters",
      themeMode: "theme mode",
      languageMode: "language",
    },
    status: {
      backend: "Backend",
      stream: "Stream",
      notifications: "Notifications",
      lastScan: "Last scan",
      symbols: "Symbols",
    },
    states: {
      connected: "connected",
      disconnected: "disconnected",
      idle: "idle",
      unsupported: "unsupported",
      granted: "granted",
      denied: "denied",
      default: "default",
    },
    filters: {
      all: "All",
      trend: "Trend",
      range: "Range",
      hot: "Hot",
      fixed: "Fixed",
    },
    themes: {
      light: "Light",
      dark: "Dark",
      system: "System",
    },
    languages: {
      zh: "中文",
      en: "English",
    },
    actions: {
      enableNotifications: "Enable notifications",
    },
    empty: {
      title: "No symbols loaded",
      body: "Start the Rust backend to populate the radar.",
    },
    table: {
      symbol: "Symbol",
      price: "Price",
      trend: "Trend",
      range: "Range",
      signal: "Signal",
    },
    detail: {
      noActiveTrigger: "No active trigger",
      funding: "Funding",
      updated: "Updated",
      fvg: "FVG",
      noFvgZones: "No FVG zones",
      levels: "Levels",
      noLevels: "No levels",
      distance: "dist",
      touches: "touches",
      account: "Account",
      noApiKey: "No read-only OKX API key connected.",
    },
    misc: {
      unlabeled: "unlabeled",
      watching: "watching",
    },
    directions: {
      long: "long",
      short: "short",
      neutral: "neutral",
    },
    poolTags: {
      dynamic: "dynamic",
      fixed: "fixed",
      manual_watch: "manual watch",
      new_listing: "new listing",
      thin_history: "thin history",
      low_market_cap: "low market cap",
      volume_mcap_anomaly: "volume/mcap anomaly",
      derivative_dominated: "derivative dominated",
      identity_uncertain: "identity uncertain",
    },
    levelKinds: {
      support: "support",
      resistance: "resistance",
    },
  },
} as const;

type WidenStrings<T> = {
  readonly [Key in keyof T]: T[Key] extends Record<string, unknown>
    ? WidenStrings<T[Key]>
    : string;
};

export type Copy = WidenStrings<(typeof translations)["en"]>;
