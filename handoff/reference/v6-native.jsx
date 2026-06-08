// Variants tuned to user's actual terminal (Tokyo Night-ish).
// env tag removed per user request — profile name + account + region + role is enough.

const TN_BG       = "#1a1b26";
const TN_FG       = "#c0caf5";
const TN_DIM      = "#565f89";
const TN_MUTED    = "#787c99";
const TN_MINT     = "#73daca";
const TN_PINK     = "#f7768e";
const TN_PURPLE   = "#bb9af7";
const TN_BLUE     = "#7aa2f7";
const TN_CYAN     = "#7dcfff";
const TN_GREEN    = "#9ece6a";
const TN_YELLOW   = "#e0af68";
const TN_ORANGE   = "#ff9e64";
const TN_RED      = "#f7768e";

function NativeTerm({ children, command = "awsp", title = "~/projects/nomadsre/awsp" }) {
  return (
    <div style={{
      width: "100%",
      height: "100%",
      background: TN_BG,
      borderRadius: 10,
      overflow: "hidden",
      display: "flex",
      flexDirection: "column",
      boxShadow: "0 30px 60px -20px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.04)",
      fontFamily: "'JetBrains Mono', 'Menlo', 'Consolas', monospace",
      color: TN_FG,
      fontSize: 13.5,
      lineHeight: 1.5,
    }}>
      <div style={{
        background: "linear-gradient(#2a2b3d, #1f2030)",
        padding: "8px 12px",
        display: "flex", alignItems: "center", gap: 6,
        borderBottom: "1px solid #0d0d14",
        flexShrink: 0,
      }}>
        <span style={{ width: 11, height: 11, borderRadius: 999, background: "#ff5f57" }} />
        <span style={{ width: 11, height: 11, borderRadius: 999, background: "#febc2e" }} />
        <span style={{ width: 11, height: 11, borderRadius: 999, background: "#28c840" }} />
      </div>
      <div style={{ padding: "14px 16px", overflow: "hidden", whiteSpace: "pre" }}>
        <div>
          <span style={{ color: TN_MINT }}>{title}</span>
          <span style={{ color: TN_FG }}> </span>
          <span style={{ color: TN_MINT }}>[</span>
          <span style={{ color: TN_MINT }}>main</span>
          <span style={{ color: TN_PINK }}>*</span>
          <span style={{ color: TN_MINT }}>]</span>
        </div>
        <div style={{ paddingBottom: 6 }}>
          <span style={{ color: TN_PINK }}>» </span>
          <span style={{ color: TN_FG }}>{command}</span>
        </div>
        {children}
      </div>
    </div>
  );
}

function S({ c, b, u, children, bg }) {
  return <span style={{ color: c, fontWeight: b ? 700 : 400, textDecoration: u ? "underline" : "none", background: bg }}>{children}</span>;
}

// ────────────────────────────────────────────────────────────
// F · Numbered hotkeys — fastest for power users
// ────────────────────────────────────────────────────────────
function VariantNumbered() {
  const sel = 4;
  return (
    <NativeTerm>
      <div style={{ paddingTop: 4 }}>
        <S c={TN_MINT}>●</S><S c={TN_DIM}> active </S><S c={TN_FG} b>acme-staging-dev</S>
        <S c={TN_DIM}>  ·  </S><S c={TN_MUTED}>447091823641</S>
        <S c={TN_DIM}>  ·  </S><S c={TN_CYAN}>us-west-2</S>
        <S c={TN_DIM}>  ·  </S><S c={TN_PURPLE}>DeveloperAccess</S>
        <S c={TN_DIM}>  ·  </S><S c={TN_GREEN}>3h 12m</S>
      </div>
      <div style={{ height: 8 }} />
      {PROFILES.map((p, i) => {
        const selected = i === sel;
        const isCur = p.name === CURRENT;
        return (
          <div key={p.name}>
            <S c={selected ? TN_PINK : TN_DIM} b={selected}>{selected ? "▸" : " "}</S>
            <S c={TN_PINK} b>{` ${i+1} `}</S>
            <S c={isCur ? TN_GREEN : TN_FG} b={selected}>{p.name.padEnd(24, " ")}</S>
            <S c={TN_MUTED}>{p.account.padEnd(15, " ")}</S>
            <S c={TN_CYAN}>{p.region.padEnd(16, " ")}</S>
            <S c={TN_PURPLE}>{p.role}</S>
            {isCur && <S c={TN_GREEN}>{"  ◀ current"}</S>}
          </div>
        );
      })}
      <div style={{ height: 6 }} />
      <div>
        <S c={TN_DIM}>  </S>
        <S c={TN_PINK} b>1-9</S><S c={TN_DIM}> jump  </S>
        <S c={TN_PINK} b>↑↓</S><S c={TN_DIM}> nav  </S>
        <S c={TN_PINK} b>/</S><S c={TN_DIM}> filter  </S>
        <S c={TN_PINK} b>⏎</S><S c={TN_DIM}> switch  </S>
        <S c={TN_PINK} b>r</S><S c={TN_DIM}> re-login  </S>
        <S c={TN_PINK} b>q</S><S c={TN_DIM}> quit</S>
      </div>
    </NativeTerm>
  );
}

// ────────────────────────────────────────────────────────────
// G · Compact aligned table — every column scannable
// ────────────────────────────────────────────────────────────
function VariantTable() {
  const sel = 4;
  return (
    <NativeTerm>
      <div style={{ paddingTop: 4 }}>
        <S c={TN_DIM}>  </S>
        <S c={TN_MUTED} b>{"PROFILE".padEnd(26, " ")}</S>
        <S c={TN_MUTED} b>{"ACCOUNT".padEnd(15, " ")}</S>
        <S c={TN_MUTED} b>{"REGION".padEnd(18, " ")}</S>
        <S c={TN_MUTED} b>{"ROLE"}</S>
      </div>
      <div>
        <S c={TN_DIM}>{"  " + "─".repeat(76)}</S>
      </div>
      {PROFILES.map((p, i) => {
        const selected = i === sel;
        const isCur = p.name === CURRENT;
        return (
          <div key={p.name} style={{
            background: selected ? "#252638" : "transparent",
          }}>
            <S c={selected ? TN_PINK : (isCur ? TN_GREEN : TN_DIM)} b>{selected ? "▸ " : (isCur ? "● " : "  ")}</S>
            <S c={isCur ? TN_GREEN : TN_FG} b={selected}>{p.name.padEnd(24, " ")}</S>
            <S c={TN_MUTED}>{p.account.padEnd(15, " ")}</S>
            <S c={TN_CYAN}>{p.region.padEnd(18, " ")}</S>
            <S c={TN_PURPLE}>{p.role}</S>
          </div>
        );
      })}
      <div style={{ height: 6 }} />
      <div>
        <S c={TN_DIM}>  10 profiles · </S>
        <S c={TN_GREEN}>✓ SSO valid</S>
        <S c={TN_DIM}> · expires </S>
        <S c={TN_YELLOW}>3h 12m</S>
        <S c={TN_DIM}>  ────  </S>
        <S c={TN_PINK} b>↑↓</S><S c={TN_DIM}>·</S>
        <S c={TN_PINK} b>⏎</S><S c={TN_DIM}>·</S>
        <S c={TN_PINK} b>/</S><S c={TN_DIM}>·</S>
        <S c={TN_PINK} b>q</S>
      </div>
    </NativeTerm>
  );
}

// ────────────────────────────────────────────────────────────
// I · `awsp status` + ambiguous match
// ────────────────────────────────────────────────────────────
function VariantStatus() {
  return (
    <NativeTerm command="awsp status">
      <div style={{ paddingTop: 4 }}>
        <S c={TN_MINT}>●</S>
        <S c={TN_FG} b>{" acme-staging-dev "}</S>
        <S c={TN_DIM}>{"  ·  "}</S>
        <S c={TN_CYAN}>us-west-2</S>
        <S c={TN_DIM}>{"  ·  "}</S>
        <S c={TN_PURPLE}>DeveloperAccess</S>
        <S c={TN_DIM}>{"  ·  "}</S>
        <S c={TN_GREEN}>valid 3h 12m</S>
      </div>
      <div>
        <S c={TN_DIM}>{"  └─ "}</S>
        <S c={TN_MUTED}>{"447091823641 · acme-corp.awsapps.com/start"}</S>
      </div>
      <div style={{ height: 8 }} />
      <div><S c={TN_MINT}>~/projects/nomadsre/awsp </S><S c={TN_MINT}>[</S><S c={TN_MINT}>main</S><S c={TN_PINK}>*</S><S c={TN_MINT}>]</S></div>
      <div><S c={TN_PINK}>» </S><S c={TN_FG}>awsp prod</S></div>
      <div style={{ paddingTop: 4 }}>
        <S c={TN_DIM}>  matches 3 profiles:</S>
      </div>
      <div>
        <S c={TN_PINK}>{"  1 "}</S>
        <S c={TN_FG} b>{"acme-prod-admin     "}</S>
        <S c={TN_MUTED}>{"682471093210  "}</S>
        <S c={TN_CYAN}>{"us-east-1  "}</S>
        <S c={TN_PURPLE}>{"AdministratorAccess"}</S>
      </div>
      <div>
        <S c={TN_PINK}>{"  2 "}</S>
        <S c={TN_FG} b>{"acme-prod-readonly  "}</S>
        <S c={TN_MUTED}>{"682471093210  "}</S>
        <S c={TN_CYAN}>{"us-east-1  "}</S>
        <S c={TN_PURPLE}>{"ReadOnlyAccess"}</S>
      </div>
      <div>
        <S c={TN_PINK}>{"  3 "}</S>
        <S c={TN_FG} b>{"acme-prod-billing   "}</S>
        <S c={TN_MUTED}>{"682471093210  "}</S>
        <S c={TN_CYAN}>{"us-east-1  "}</S>
        <S c={TN_PURPLE}>{"BillingAccess"}</S>
      </div>
      <div style={{ height: 4 }} />
      <div>
        <S c={TN_DIM}>{"  pick "}</S>
        <S c={TN_PINK} b>1-3</S>
        <S c={TN_DIM}>{" · or refine: "}</S>
        <S c={TN_FG}>awsp prod-r</S>
        <span style={{ background: TN_FG, width: 7, height: 14, display: "inline-block", verticalAlign: "-2px", marginLeft: 1, animation: "blk 1s steps(1) infinite" }} />
      </div>
    </NativeTerm>
  );
}

// ────────────────────────────────────────────────────────────
// J · After-switch result
// ────────────────────────────────────────────────────────────
function VariantSuccess() {
  return (
    <NativeTerm command="awsp prod-readonly">
      <div style={{ paddingTop: 4 }}>
        <S c={TN_GREEN} b>{"  ✓  "}</S>
        <S c={TN_DIM}>{"switched  "}</S>
        <S c={TN_FG} b>{"acme-staging-dev"}</S>
        <S c={TN_DIM}>{"  →  "}</S>
        <S c={TN_FG} b>{"acme-prod-readonly"}</S>
      </div>
      <div>
        <S c={TN_DIM}>{"     "}</S>
        <S c={TN_MUTED}>{"682471093210 · us-east-1 · ReadOnlyAccess · session 7h 58m"}</S>
      </div>
      <div style={{ height: 6 }} />
      <div>
        <S c={TN_MINT}>~/projects/nomadsre/awsp </S>
        <S c={TN_MINT}>[</S><S c={TN_MINT}>main</S><S c={TN_PINK}>*</S><S c={TN_MINT}>]</S>
        <S c={TN_DIM}>{"  "}</S>
        <S c={TN_RED}>{"⌁ "}</S>
        <S c={TN_RED} b>{"prod-readonly"}</S>
      </div>
      <div>
        <S c={TN_PINK}>» </S>
        <span style={{ background: TN_FG, width: 7, height: 14, display: "inline-block", verticalAlign: "-2px", animation: "blk 1s steps(1) infinite" }} />
      </div>
      <style>{`@keyframes blk { 0%, 49% { opacity: 1 } 50%, 100% { opacity: 0 } }`}</style>
    </NativeTerm>
  );
}

Object.assign(window, {
  VariantNumbered, VariantTable,
  VariantStatusNative: VariantStatus,
  VariantSuccessNative: VariantSuccess,
});
