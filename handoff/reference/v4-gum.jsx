// Variant 4 — gum-style: airy, opinionated, very little chrome.
// Big legible rows, single accent color, generous spacing.

function VariantGum() {
  const sel = 4;
  return (
    <Terminal title="awsp">
      <div style={{ padding: "8px 4px 4px" }}>
        <T c={TERM_ACCENT} b style={{ fontSize: 15 }}>{"▌"}</T>
        <T c={TERM_FG} b style={{ fontSize: 15 }}>{"  Choose a profile"}</T>
      </div>
      <div style={{ color: TERM_DIM, padding: "0 4px 14px 4px", fontSize: 12 }}>
        Currently active: <T c={TERM_GREEN} b>{CURRENT}</T>
      </div>

      <div>
        {PROFILES.slice(0, 8).map((p, i) => {
          const selected = i === sel;
          return (
            <div key={p.name} style={{
              padding: "6px 10px",
              margin: "1px 0",
              background: selected ? "#1a1a1d" : "transparent",
              borderRadius: 3,
              display: "flex",
              alignItems: "center",
              gap: 10,
            }}>
              <T c={selected ? TERM_ACCENT : "transparent"} b>{selected ? "▸" : "·"}</T>
              <T c={selected ? TERM_FG : TERM_MUTED} b={selected} style={{ fontSize: selected ? 14.5 : 13.5, minWidth: 220 }}>
                {p.name}
              </T>
              <span style={{
                background: ENV_COLOR[p.env] + "22",
                color: ENV_COLOR[p.env],
                padding: "1px 8px",
                borderRadius: 999,
                fontSize: 11,
                fontWeight: 600,
              }}>{p.env}</span>
              <T c={TERM_DIM} style={{ fontSize: 12, flex: 1, textAlign: "right" }}>{p.region}</T>
            </div>
          );
        })}
      </div>

      <div style={{ padding: "16px 4px 0", color: TERM_DIM, fontSize: 12 }}>
        <T c={TERM_ACCENT}>↑↓</T> navigate  ·  <T c={TERM_ACCENT}>enter</T> select  ·  <T c={TERM_ACCENT}>/</T> filter  ·  <T c={TERM_ACCENT}>esc</T> cancel
      </div>
    </Terminal>
  );
}
window.VariantGum = VariantGum;
