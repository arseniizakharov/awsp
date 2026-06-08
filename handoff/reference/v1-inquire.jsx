// Variant 1 — inquire-style interactive picker
// Single-column with prompt, fuzzy filter, current marker. Dense info per row.

function VariantInquire() {
  const filtered = PROFILES;
  const selectedIdx = 4; // acme-staging-dev (matches CURRENT)
  return (
    <Terminal title="awsp — inquire">
      <style>{cursorCss}</style>
      <div>
        <T c={TERM_GREEN} b>{"? "}</T>
        <T c={TERM_FG} b>Switch AWS profile</T>
        <T c={TERM_DIM}>  (type to filter · ↑↓ to move · enter to select · esc to quit)</T>
      </div>
      <div style={{ marginTop: 2 }}>
        <T c={TERM_ACCENT_2} b>{"› "}</T>
        <T c={TERM_FG}>acme</T>
        <T c={TERM_FG} style={{ background: "#3a3a1c" }}>staging</T>
        <span className="blk" style={{ background: TERM_FG, width: 8, height: 15, display: "inline-block", verticalAlign: "-2px" }} />
      </div>
      <div style={{ marginTop: 12 }}>
        {filtered.map((p, i) => {
          const sel = i === selectedIdx;
          const matchA = "acme-";
          const matchB = "staging" in {} ? "" : p.name.includes("staging") ? "staging" : "";
          return (
            <div key={p.name} style={{
              background: sel ? "#1a1f28" : "transparent",
              padding: "1px 0 1px 0",
              borderLeft: sel ? `2px solid ${TERM_ACCENT}` : "2px solid transparent",
              paddingLeft: 8,
            }}>
              <T c={sel ? TERM_ACCENT : "transparent"} b>{sel ? "▸ " : "  "}</T>
              <T c={p.name === CURRENT ? TERM_GREEN : TERM_FG} b={sel}>
                {p.name.padEnd(24, " ")}
              </T>
              <T c={TERM_DIM}>{"  "}</T>
              <T c={ENV_COLOR[p.env]}>{("[" + p.env + "]").padEnd(10, " ")}</T>
              <T c={TERM_MUTED}>{p.account}</T>
              <T c={TERM_DIM}>{"  ·  "}</T>
              <T c={TERM_MUTED}>{p.role.padEnd(22, " ")}</T>
              <T c={TERM_DIM}>{p.region}</T>
              {p.name === CURRENT && <T c={TERM_GREEN} b>{"  ●  current"}</T>}
            </div>
          );
        })}
      </div>
      <div style={{ marginTop: 14, color: TERM_DIM, fontSize: 12 }}>
        <T c={TERM_DIM}>{"────────────────────────────────────────────────────────────────────────"}</T>
        {"\n"}
        <T c={TERM_MUTED} b>{"acme-staging-dev"}</T>
        <T c={TERM_DIM}>{"  ·  447091823641  ·  us-west-2  ·  SSO: acme-corp"}</T>
        {"\n"}
        <T c={TERM_DIM}>{"Session expires in 3h 12m. Press "}</T>
        <T c={TERM_ACCENT}>{"⏎"}</T>
        <T c={TERM_DIM}>{" to switch  ·  "}</T>
        <T c={TERM_ACCENT}>{"^L"}</T>
        <T c={TERM_DIM}>{" re-login  ·  "}</T>
        <T c={TERM_ACCENT}>{"?"}</T>
        <T c={TERM_DIM}>{" help"}</T>
      </div>
    </Terminal>
  );
}
window.VariantInquire = VariantInquire;
