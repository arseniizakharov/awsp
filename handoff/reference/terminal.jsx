// Terminal frame primitive — mac-style window chrome wrapping a monospace canvas.
// All variants render into <Terminal>. Use <T> for styled spans (Box-drawing safe).

const TERM_BG = "#0e0f12";
const TERM_FG = "#d8d6cf";
const TERM_DIM = "#6b6b6b";
const TERM_MUTED = "#8a8a8a";
const TERM_ACCENT = "#e8a04a";    // amber — "cloud tooling" vibe, NOT AWS orange
const TERM_ACCENT_2 = "#7aa7d6";  // cool blue accent
const TERM_GREEN = "#8ec07c";
const TERM_RED = "#e07b7b";
const TERM_MAGENTA = "#c792ea";

function Terminal({ title = "~/awsp", cols = 92, rows = 28, children, padded = true, style = {} }) {
  return (
    <div style={{
      width: "100%",
      height: "100%",
      background: "#1c1c1f",
      borderRadius: 10,
      overflow: "hidden",
      display: "flex",
      flexDirection: "column",
      boxShadow: "0 30px 60px -20px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.04)",
      fontFamily: "'JetBrains Mono', 'Menlo', 'Consolas', monospace",
      color: TERM_FG,
      ...style,
    }}>
      {/* chrome */}
      <div style={{
        display: "flex",
        alignItems: "center",
        padding: "10px 14px",
        background: "linear-gradient(#26262a,#1c1c1f)",
        borderBottom: "1px solid #000",
        gap: 8,
        flexShrink: 0,
      }}>
        <span style={{ width: 12, height: 12, borderRadius: 999, background: "#ff5f57" }} />
        <span style={{ width: 12, height: 12, borderRadius: 999, background: "#febc2e" }} />
        <span style={{ width: 12, height: 12, borderRadius: 999, background: "#28c840" }} />
        <div style={{
          flex: 1,
          textAlign: "center",
          fontSize: 12,
          color: "#9a9a9a",
          fontFamily: "system-ui, sans-serif",
        }}>{title}</div>
        <div style={{ width: 44 }} />
      </div>
      {/* body */}
      <div style={{
        flex: 1,
        background: TERM_BG,
        padding: padded ? "16px 18px 18px" : 0,
        fontSize: 13.5,
        lineHeight: 1.55,
        overflow: "hidden",
        whiteSpace: "pre",
        letterSpacing: 0.1,
      }}>
        {children}
      </div>
    </div>
  );
}

// Inline styled span — keeps monospace alignment.
function T({ c, b, u, children, style = {} }) {
  const s = { color: c, fontWeight: b ? 700 : 400, textDecoration: u ? "underline" : "none", ...style };
  return <span style={s}>{children}</span>;
}

// Cursor blink
const cursorCss = `
@keyframes blk { 0%, 49% { opacity: 1 } 50%, 100% { opacity: 0 } }
.blk { animation: blk 1s steps(1) infinite; }
`;

Object.assign(window, {
  Terminal, T,
  TERM_BG, TERM_FG, TERM_DIM, TERM_MUTED,
  TERM_ACCENT, TERM_ACCENT_2, TERM_GREEN, TERM_RED, TERM_MAGENTA,
  cursorCss,
});
