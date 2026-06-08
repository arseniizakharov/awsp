// Variant 2 — ratatui-style boxed dashboard
// Multiple panels: list, current session detail, recent activity. Box drawing chars.

function VariantRatatui() {
  const sel = 4; // acme-staging-dev
  // Box-drawing helpers
  const H = "─", V = "│", TL = "╭", TR = "╮", BL = "╰", BR = "╯";
  const TT = "┬", BT = "┴", LT = "├", RT = "┤", X = "┼";

  const sessionExpires = "3h 12m";

  return (
    <Terminal title="awsp — dashboard" cols={100}>
      {/* Title bar */}
      <div style={{ display: "flex", marginBottom: 6 }}>
        <T c={TERM_ACCENT} b>{" ☁ awsp "}</T>
        <T c={TERM_DIM}>{"  v0.4.2"}</T>
        <T c={TERM_DIM}>{"  ·  "}</T>
        <T c={TERM_MUTED}>{"10 profiles  ·  3 SSO orgs"}</T>
        <span style={{ flex: 1 }} />
        <T c={TERM_GREEN}>{"● "}</T>
        <T c={TERM_FG}>{"signed in"}</T>
        <T c={TERM_DIM}>{"  ·  expires in "}</T>
        <T c={TERM_ACCENT}>{sessionExpires}</T>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
        {/* LEFT: profile list with grouping */}
        <Box title="Profiles" hint="↑↓ /:filter ⏎:switch">
          <Group label="acme-corp" color={TERM_ACCENT} />
          {PROFILES.slice(0,7).map((p, i) => (
            <Row key={p.name} p={p} sel={i === sel} isCurrent={p.name === CURRENT} />
          ))}
          <Group label="personal" color={TERM_MAGENTA} />
          {PROFILES.slice(7,8).map(p => <Row key={p.name} p={p} />)}
          <Group label="clients" color={TERM_GREEN} />
          {PROFILES.slice(8).map(p => <Row key={p.name} p={p} />)}
        </Box>

        {/* RIGHT: stacked detail + recent */}
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <Box title="Selected" hint="⏎ to activate">
            <div style={{ padding: "2px 0 4px" }}>
              <T c={TERM_ACCENT} b>{"acme-staging-dev"}</T>
              <T c={TERM_DIM}>{"  "}</T>
              <T c={ENV_COLOR.staging}>{"[staging]"}</T>
            </div>
            <KV k="Account"  v="447091823641" />
            <KV k="Role"     v="DeveloperAccess" vColor={TERM_ACCENT_2} />
            <KV k="Region"   v="us-west-2" />
            <KV k="SSO"      v="acme-corp.awsapps.com/start" vColor={TERM_MUTED} />
            <KV k="Session"  v="valid · 3h 12m remaining" vColor={TERM_GREEN} />
            <div style={{ marginTop: 6 }}>
              <T c={TERM_DIM}>{"Console URL  "}</T>
              <T c={TERM_ACCENT_2} u>{"https://447091823641.console.signin..."}</T>
            </div>
          </Box>
          <Box title="Recent" hint="^R to repeat">
            <RecentRow when="2m ago"   action="switch" name="acme-staging-dev"   ok />
            <RecentRow when="38m ago"  action="switch" name="acme-prod-readonly" ok />
            <RecentRow when="1h ago"   action="login " name="acme-corp SSO"      ok />
            <RecentRow when="2h ago"   action="switch" name="client-northwind"   ok />
            <RecentRow when="yesterday" action="logout" name="all sessions"      ok />
          </Box>
        </div>
      </div>

      {/* Bottom status bar */}
      <div style={{ marginTop: 10, background: "#15171b", padding: "4px 8px", borderRadius: 3 }}>
        <T c={TERM_ACCENT} b>{" NORMAL "}</T>
        <T c={TERM_DIM}>{"  "}</T>
        <T c={TERM_FG}>{"↑↓"}</T><T c={TERM_DIM}>{" move  "}</T>
        <T c={TERM_FG}>{"/"}</T><T c={TERM_DIM}>{" filter  "}</T>
        <T c={TERM_FG}>{"⏎"}</T><T c={TERM_DIM}>{" switch  "}</T>
        <T c={TERM_FG}>{"e"}</T><T c={TERM_DIM}>{" export  "}</T>
        <T c={TERM_FG}>{"L"}</T><T c={TERM_DIM}>{" login  "}</T>
        <T c={TERM_FG}>{"q"}</T><T c={TERM_DIM}>{" quit"}</T>
      </div>
    </Terminal>
  );
}

function Box({ title, hint, children }) {
  return (
    <div style={{ position: "relative", border: "1px solid #2a2d33", borderRadius: 4, padding: "6px 10px 8px", background: "#101216" }}>
      <div style={{ position: "absolute", top: -8, left: 10, background: TERM_BG, padding: "0 6px", fontSize: 12 }}>
        <T c={TERM_ACCENT} b>{title}</T>
      </div>
      {hint && <div style={{ position: "absolute", top: -8, right: 10, background: TERM_BG, padding: "0 6px", fontSize: 11 }}>
        <T c={TERM_DIM}>{hint}</T>
      </div>}
      <div style={{ marginTop: 4 }}>{children}</div>
    </div>
  );
}

function Group({ label, color }) {
  return (
    <div style={{ marginTop: 4 }}>
      <T c={color}>{"▾ "}</T>
      <T c={TERM_MUTED} b>{label}</T>
    </div>
  );
}

function Row({ p, sel, isCurrent }) {
  return (
    <div style={{
      background: sel ? "#1c2230" : "transparent",
      paddingLeft: 16,
      borderLeft: sel ? `2px solid ${TERM_ACCENT}` : "2px solid transparent",
    }}>
      <T c={isCurrent ? TERM_GREEN : (sel ? TERM_FG : TERM_FG)} b={sel}>
        {isCurrent ? "● " : (sel ? "▸ " : "  ")}
      </T>
      <T c={isCurrent ? TERM_GREEN : TERM_FG} b={sel}>{p.name.padEnd(22, " ")}</T>
      <T c={TERM_DIM}>{p.region.padEnd(16, " ")}</T>
      <T c={ENV_COLOR[p.env]}>{p.env}</T>
    </div>
  );
}

function KV({ k, v, vColor = TERM_FG }) {
  return (
    <div>
      <T c={TERM_DIM}>{(k + ":").padEnd(10, " ")}</T>
      <T c={vColor}>{v}</T>
    </div>
  );
}

function RecentRow({ when, action, name, ok }) {
  return (
    <div>
      <T c={ok ? TERM_GREEN : TERM_RED}>{ok ? "✓ " : "✗ "}</T>
      <T c={TERM_MUTED}>{action}</T>
      <T c={TERM_DIM}>{"  "}</T>
      <T c={TERM_FG}>{name.padEnd(24, " ")}</T>
      <T c={TERM_DIM}>{when}</T>
    </div>
  );
}

window.VariantRatatui = VariantRatatui;
