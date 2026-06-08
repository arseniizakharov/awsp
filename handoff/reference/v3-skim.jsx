// Variant 3 — skim/fzf-style: list + preview pane, fuzzy match with highlighted chars
// Bottom-anchored prompt like fzf.

function VariantSkim() {
  // Simulate fuzzy match of "stg" against names — highlight matched chars
  const query = "stg";
  const match = (name) => {
    const out = [];
    let qi = 0;
    for (let i = 0; i < name.length; i++) {
      if (qi < query.length && name[i].toLowerCase() === query[qi].toLowerCase()) {
        out.push({ c: name[i], m: true });
        qi++;
      } else out.push({ c: name[i], m: false });
    }
    return qi === query.length ? out : null;
  };

  const filtered = PROFILES.map(p => ({ p, m: match(p.name) })).filter(x => x.m);
  const sel = 0;
  const cur = filtered[sel].p;

  return (
    <Terminal title="awsp — fuzzy">
      <style>{cursorCss}</style>
      <div style={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 0, height: "calc(100% - 60px)" }}>
        {/* LEFT: results */}
        <div style={{ paddingRight: 12, borderRight: "1px solid #25272c", display: "flex", flexDirection: "column" }}>
          <div style={{ flex: 1 }}>
            {filtered.map((x, i) => {
              const selected = i === sel;
              return (
                <div key={x.p.name} style={{
                  background: selected ? "#1a2230" : "transparent",
                  paddingLeft: 6,
                  borderLeft: selected ? `2px solid ${TERM_ACCENT}` : "2px solid transparent",
                }}>
                  <T c={selected ? TERM_ACCENT : "transparent"} b>{selected ? "▸ " : "  "}</T>
                  {x.m.map((ch, j) => (
                    <T key={j} c={ch.m ? TERM_ACCENT : TERM_FG} b={ch.m}>{ch.c}</T>
                  ))}
                  <T c={TERM_DIM}>{" ".repeat(Math.max(2, 26 - x.p.name.length))}</T>
                  <T c={ENV_COLOR[x.p.env]}>{x.p.env}</T>
                </div>
              );
            })}
          </div>
          <div style={{ paddingTop: 8 }}>
            <T c={TERM_DIM}>{`  ${filtered.length}/${PROFILES.length}`}</T>
          </div>
        </div>

        {/* RIGHT: preview pane */}
        <div style={{ paddingLeft: 14 }}>
          <T c={TERM_DIM} b>{"╭─ preview ────────────────────────────╮"}</T>{"\n"}
          <T c={TERM_FG} b>{cur.name}</T>{"\n"}
          <T c={TERM_DIM}>{"─".repeat(40)}</T>{"\n"}
          <T c={TERM_MUTED}>{"account  "}</T><T c={TERM_FG}>{cur.account}</T>{"\n"}
          <T c={TERM_MUTED}>{"role     "}</T><T c={TERM_ACCENT_2}>{cur.role}</T>{"\n"}
          <T c={TERM_MUTED}>{"region   "}</T><T c={TERM_FG}>{cur.region}</T>{"\n"}
          <T c={TERM_MUTED}>{"sso      "}</T><T c={TERM_FG}>{cur.sso}</T>{"\n"}
          <T c={TERM_MUTED}>{"env      "}</T><T c={ENV_COLOR[cur.env]}>{cur.env}</T>{"\n"}
          {"\n"}
          <T c={TERM_DIM}>{"# config block"}</T>{"\n"}
          <T c={TERM_MAGENTA}>{`[profile ${cur.name}]`}</T>{"\n"}
          <T c={TERM_ACCENT_2}>{"sso_session     "}</T><T c={TERM_FG}>{cur.sso}</T>{"\n"}
          <T c={TERM_ACCENT_2}>{"sso_account_id  "}</T><T c={TERM_FG}>{cur.account}</T>{"\n"}
          <T c={TERM_ACCENT_2}>{"sso_role_name   "}</T><T c={TERM_FG}>{cur.role}</T>{"\n"}
          <T c={TERM_ACCENT_2}>{"region          "}</T><T c={TERM_FG}>{cur.region}</T>{"\n"}
          <T c={TERM_ACCENT_2}>{"output          "}</T><T c={TERM_FG}>{"json"}</T>{"\n"}
          {"\n"}
          <T c={TERM_GREEN}>{"✓ "}</T><T c={TERM_DIM}>{"credentials valid · 3h 12m left"}</T>
        </div>
      </div>

      {/* Bottom prompt — fzf style */}
      <div style={{ marginTop: 8, borderTop: "1px solid #25272c", paddingTop: 8 }}>
        <T c={TERM_DIM}>{"  "}</T>
        <T c={TERM_ACCENT_2} b>{"❯ "}</T>
        <T c={TERM_FG}>{query}</T>
        <span className="blk" style={{ background: TERM_FG, width: 7, height: 14, display: "inline-block", verticalAlign: "-2px" }} />
        <span style={{ float: "right" }}>
          <T c={TERM_DIM}>{"tab:multi  ctrl-r:reload  ctrl-y:copy id  ?:help"}</T>
        </span>
      </div>
    </Terminal>
  );
}
window.VariantSkim = VariantSkim;
