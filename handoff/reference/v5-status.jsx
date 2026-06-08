// Variant 5 — Status / result screens (post-selection moments)
// Three little vignettes stacked: success, expired SSO + login flow, error.

function VariantStatus() {
  return (
    <Terminal title="awsp — flows" padded={false}>
      <div style={{ padding: "14px 18px", display: "flex", flexDirection: "column", gap: 14 }}>
        {/* Success */}
        <div>
          <T c={TERM_DIM}>{"$ "}</T>
          <T c={TERM_FG} b>{"awsp"}</T>
          <T c={TERM_DIM}>{" acme-prod-readonly"}</T>{"\n"}
          <T c={TERM_GREEN} b>{"  ✓ "}</T>
          <T c={TERM_FG}>{"Switched to "}</T>
          <T c={TERM_ACCENT} b>{"acme-prod-readonly"}</T>{"\n"}
          <T c={TERM_DIM}>{"     account  "}</T><T c={TERM_FG}>{"682471093210"}</T>{"\n"}
          <T c={TERM_DIM}>{"     role     "}</T><T c={TERM_ACCENT_2}>{"ReadOnlyAccess"}</T>{"\n"}
          <T c={TERM_DIM}>{"     region   "}</T><T c={TERM_FG}>{"us-east-1"}</T>{"\n"}
          <T c={TERM_DIM}>{"     env      "}</T>
          <span style={{ background: ENV_COLOR.prod + "22", color: ENV_COLOR.prod, padding: "0 6px", borderRadius: 3, fontSize: 11, fontWeight: 600 }}>PROD</span>{"\n"}
          <T c={TERM_DIM}>{"  → "}</T>
          <T c={TERM_MUTED}>{"exported AWS_PROFILE, AWS_REGION  ·  session ok for 7h 58m"}</T>
        </div>

        <Hr />

        {/* Expired session → device flow */}
        <div>
          <T c={TERM_DIM}>{"$ "}</T>
          <T c={TERM_FG} b>{"awsp"}</T>
          <T c={TERM_DIM}>{" acme-prod-admin"}</T>{"\n"}
          <T c={TERM_RED} b>{"  ! "}</T>
          <T c={TERM_FG}>{"SSO session for "}</T>
          <T c={TERM_ACCENT}>{"acme-corp"}</T>
          <T c={TERM_FG}>{" expired"}</T>
          <T c={TERM_DIM}>{"  (4d ago)"}</T>{"\n"}
          <T c={TERM_DIM}>{"  → "}</T>
          <T c={TERM_FG}>{"Launching device authorization…"}</T>{"\n"}
          {"\n"}
          <div style={{ border: "1px solid #2a2d33", borderRadius: 4, padding: "8px 12px", background: "#101216", marginLeft: 4 }}>
            <T c={TERM_DIM}>{"Open this URL in your browser:"}</T>{"\n"}
            <T c={TERM_ACCENT_2} u b>{"  https://device.sso.us-east-1.amazonaws.com/"}</T>{"\n"}
            {"\n"}
            <T c={TERM_DIM}>{"Confirm the code: "}</T>
            <T c={TERM_ACCENT} b style={{ fontSize: 16, letterSpacing: 2 }}>{"WXKQ-MTRP"}</T>{"\n"}
            <T c={TERM_DIM}>{"Waiting for confirmation "}</T>
            <T c={TERM_ACCENT}>{"⠋"}</T>
            <T c={TERM_DIM}>{"  (auto-retry, ctrl-c to cancel)"}</T>
          </div>
        </div>

        <Hr />

        {/* Error / not found with suggestion */}
        <div>
          <T c={TERM_DIM}>{"$ "}</T>
          <T c={TERM_FG} b>{"awsp"}</T>
          <T c={TERM_DIM}>{" acme-prod-redonly"}</T>{"\n"}
          <T c={TERM_RED} b>{"  ✗ "}</T>
          <T c={TERM_FG}>{"No profile named "}</T>
          <T c={TERM_RED}>{"acme-prod-redonly"}</T>{"\n"}
          <T c={TERM_DIM}>{"  did you mean   "}</T>
          <T c={TERM_ACCENT} b u>{"acme-prod-readonly"}</T>
          <T c={TERM_DIM}>{"  ?"}</T>{"\n"}
          <T c={TERM_DIM}>{"     or          "}</T>
          <T c={TERM_ACCENT_2}>{"acme-prod-admin"}</T>
          <T c={TERM_DIM}>{",  "}</T>
          <T c={TERM_ACCENT_2}>{"acme-prod-billing"}</T>{"\n"}
          <T c={TERM_DIM}>{"  → "}</T>
          <T c={TERM_MUTED}>{"run "}</T>
          <T c={TERM_FG} b>{"awsp"}</T>
          <T c={TERM_MUTED}>{" with no args for the interactive picker"}</T>
        </div>
      </div>
    </Terminal>
  );
}

function Hr() {
  return <div style={{ height: 1, background: "#1f2227", margin: "2px 0" }} />;
}

window.VariantStatus = VariantStatus;
