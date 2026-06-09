use crate::aws_config::SsoProfile;
use crate::prompt;
use crate::state::{self, TeamState};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use uuid::Uuid;

const TEAM_ENDPOINT_ENV: &str = "AWSP_TEAM_GRAPHQL_ENDPOINT";
const TEAM_TOKEN_ENV: &str = "AWSP_TEAM_AUTH_TOKEN";
const TEAM_COGNITO_DOMAIN_ENV: &str = "AWSP_TEAM_COGNITO_DOMAIN";
const TEAM_CLIENT_ID_ENV: &str = "AWSP_TEAM_CLIENT_ID";
const TEAM_REDIRECT_URI_ENV: &str = "AWSP_TEAM_REDIRECT_URI";
const TEAM_SCOPES_ENV: &str = "AWSP_TEAM_SCOPES";
const TEAM_IDP_IDENTIFIER_ENV: &str = "AWSP_TEAM_IDP_IDENTIFIER";
const DEFAULT_DURATION_ENV: &str = "AWSP_ELEVATE_DURATION";
const DEFAULT_TICKET_ENV: &str = "AWSP_ELEVATE_TICKET";
const DEFAULT_JUSTIFICATION_ENV: &str = "AWSP_ELEVATE_JUSTIFICATION";
const DEFAULT_TEAM_SCOPES: &str = "aws.cognito.signin.user.admin email openid phone profile";
const DEFAULT_TEAM_IDP_IDENTIFIER: &str = "team";

const GET_USER_POLICY: &str = r#"
query GetUserPolicy($userId: String, $groupIds: [String]) {
  getUserPolicy(userId: $userId, groupIds: $groupIds) {
    policy {
      accounts { name id }
      permissions { name id }
      approvalRequired
      duration
    }
  }
}
"#;

const VALIDATE_REQUEST: &str = r#"
query ValidateRequest($accountId: String!, $roleId: String!, $userId: String!, $groupIds: [String]!) {
  validateRequest(accountId: $accountId, roleId: $roleId, userId: $userId, groupIds: $groupIds) {
    valid
    reason
  }
}
"#;

const CREATE_REQUEST: &str = r#"
mutation CreateRequests($input: CreateRequestsInput!) {
  createRequests(input: $input) {
    id
    status
  }
}
"#;

#[derive(Debug, Clone, Default)]
pub struct ElevationOptions {
    pub duration_hours: Option<String>,
    pub ticket_no: Option<String>,
    pub justification: Option<String>,
    pub yes: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TeamLoginOptions {
    pub app_url: Option<String>,
    pub graphql_endpoint: Option<String>,
    pub cognito_domain: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Option<String>,
    pub idp_identifier: Option<String>,
    pub code: Option<String>,
    pub redirected_url: Option<String>,
    pub no_open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElevationOutcome {
    Submitted { id: String, status: String },
    NotConfigured,
    Declined,
}

#[derive(Debug, Clone)]
struct TeamConfig {
    endpoint: String,
    token: String,
}

#[derive(Debug, Clone)]
struct TeamAuthConfig {
    graphql_endpoint: String,
    cognito_domain: String,
    client_id: String,
    redirect_uri: String,
    scopes: String,
    idp_identifier: Option<String>,
}

#[derive(Debug, Clone)]
struct TeamAppConfig {
    graphql_endpoint: String,
    cognito_domain: String,
    client_id: String,
    redirect_uri: String,
    scopes: Option<String>,
}

#[derive(Debug, Clone)]
struct TokenResponse {
    id_token: String,
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
struct TeamIdentity {
    user_id: String,
    group_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct RequestTarget {
    account_id: String,
    account_name: String,
    role_name: String,
    role_id: String,
    max_duration: Option<String>,
}

pub fn request_access(
    profile: &SsoProfile,
    options: &ElevationOptions,
) -> Result<ElevationOutcome> {
    let Some(config) = TeamConfig::load()? else {
        explain_missing_config(profile);
        return Ok(ElevationOutcome::NotConfigured);
    };

    let identity = TeamIdentity::from_jwt(&config.token)?;
    let client = TeamClient::new(config);
    let target = client.resolve_request_target(profile, &identity)?;
    let input = collect_request_input(profile, &target, options)?;

    if !options.yes {
        let question = format!(
            "Submit TEAM request for {} / {} for {}h? [Y/n] ",
            target.account_name, target.role_name, input.duration_hours
        );
        if !prompt::yes_no(&question, true)? {
            return Ok(ElevationOutcome::Declined);
        }
    }

    client.validate_request(&target, &identity)?;
    client.create_request(&target, &input)
}

pub fn team_login(options: TeamLoginOptions) -> Result<()> {
    let auth = TeamAuthConfig::resolve(&options)?;
    let verifier = code_verifier();
    let challenge = code_challenge(&verifier);
    let state_value = Uuid::new_v4().to_string();
    let authorize_url = auth.authorize_url(&challenge, &state_value);

    eprintln!("Open this TEAM sign-in URL:");
    eprintln!("{authorize_url}");
    if !options.no_open {
        open_browser(&authorize_url);
    }

    let code = match (options.code.as_deref(), options.redirected_url.as_deref()) {
        (Some(code), _) => code.trim().to_string(),
        (_, Some(url)) => extract_authorization_code(url, Some(&state_value))?,
        (None, None) => {
            let pasted = prompt::text("Paste final redirected URL or code:", None)?;
            extract_authorization_code(&pasted, Some(&state_value)).or_else(|_| {
                if pasted.trim().is_empty() {
                    bail!("authorization code is required")
                } else {
                    Ok(pasted.trim().to_string())
                }
            })?
        }
    };

    let tokens = exchange_authorization_code(&auth, &code, &verifier)?;
    persist_team_state(&auth, tokens)?;
    eprintln!("TEAM login cached.");
    Ok(())
}

pub fn team_status() -> Result<()> {
    let Some(team) = state::get_team_state()? else {
        println!("TEAM login: not configured");
        return Ok(());
    };

    println!("TEAM GraphQL endpoint: {}", team.graphql_endpoint);
    println!("TEAM Cognito domain: {}", team.cognito_domain);
    println!("TEAM client id: {}", team.client_id);
    println!("TEAM redirect URI: {}", team.redirect_uri);
    println!("TEAM scopes: {}", team.scopes);
    println!(
        "TEAM ID token: {}",
        token_status(&team.id_token).unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "TEAM refresh token: {}",
        if team.refresh_token.is_some() {
            "cached"
        } else {
            "missing"
        }
    );
    Ok(())
}

pub fn team_logout() -> Result<()> {
    state::clear_team_state()?;
    eprintln!("Cleared cached TEAM login.");
    Ok(())
}

fn explain_missing_config(profile: &SsoProfile) {
    eprintln!(
        "  temporary elevated access is required for {} ({} / {}).",
        profile.name, profile.account_id, profile.role_name
    );
    eprintln!("  TEAM request submission is not configured for awsp yet.");
    eprintln!("  Run awsp team login, or set {TEAM_ENDPOINT_ENV} and {TEAM_TOKEN_ENV}.");
}

struct TeamClient {
    config: TeamConfig,
}

impl TeamConfig {
    fn load() -> Result<Option<Self>> {
        if let Some(config) = Self::from_env() {
            return Ok(Some(config));
        }

        let Some(team) = state::get_team_state()? else {
            return Ok(None);
        };
        if token_is_fresh(&team.id_token) {
            return Ok(Some(Self {
                endpoint: team.graphql_endpoint,
                token: team.id_token,
            }));
        }

        let Some(refresh_token) = team.refresh_token.clone() else {
            return Ok(None);
        };
        let auth = TeamAuthConfig::from_state(&team);
        let tokens = refresh_tokens(&auth, &refresh_token)?;
        persist_team_state(&auth, tokens.clone())?;
        Ok(Some(Self {
            endpoint: auth.graphql_endpoint,
            token: tokens.id_token,
        }))
    }

    fn from_env() -> Option<Self> {
        let endpoint = env::var(TEAM_ENDPOINT_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())?;
        let token = env::var(TEAM_TOKEN_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())?;

        Some(Self { endpoint, token })
    }
}

impl TeamAuthConfig {
    fn resolve(options: &TeamLoginOptions) -> Result<Self> {
        let current = state::get_team_state()?;
        let discovered = match options.app_url.as_deref() {
            Some(app_url) => Some(discover_team_app_config(app_url)?),
            None => None,
        };
        let graphql_endpoint = option_or_env_or_discovered_or_state(
            &options.graphql_endpoint,
            TEAM_ENDPOINT_ENV,
            discovered
                .as_ref()
                .map(|config| config.graphql_endpoint.as_str()),
            current.as_ref().map(|team| team.graphql_endpoint.as_str()),
        )
        .context(
            "TEAM GraphQL endpoint is required; pass --endpoint or set AWSP_TEAM_GRAPHQL_ENDPOINT",
        )?;
        let cognito_domain = option_or_env_or_discovered_or_state(
            &options.cognito_domain,
            TEAM_COGNITO_DOMAIN_ENV,
            discovered
                .as_ref()
                .map(|config| config.cognito_domain.as_str()),
            current.as_ref().map(|team| team.cognito_domain.as_str()),
        )
        .context(
            "TEAM Cognito domain is required; pass --domain or set AWSP_TEAM_COGNITO_DOMAIN",
        )?;
        let client_id = option_or_env_or_discovered_or_state(
            &options.client_id,
            TEAM_CLIENT_ID_ENV,
            discovered.as_ref().map(|config| config.client_id.as_str()),
            current.as_ref().map(|team| team.client_id.as_str()),
        )
        .context(
            "TEAM Cognito client id is required; pass --client-id or set AWSP_TEAM_CLIENT_ID",
        )?;
        let redirect_uri = option_or_env_or_discovered_or_state(
            &options.redirect_uri,
            TEAM_REDIRECT_URI_ENV,
            discovered
                .as_ref()
                .map(|config| config.redirect_uri.as_str()),
            current.as_ref().map(|team| team.redirect_uri.as_str()),
        )
        .context(
            "TEAM redirect URI is required; pass --redirect-uri or set AWSP_TEAM_REDIRECT_URI",
        )?;
        let scopes = option_or_env_or_discovered_or_state(
            &options.scopes,
            TEAM_SCOPES_ENV,
            discovered
                .as_ref()
                .and_then(|config| config.scopes.as_deref()),
            current.as_ref().map(|team| team.scopes.as_str()),
        )
        .unwrap_or_else(|| DEFAULT_TEAM_SCOPES.to_string());
        let idp_identifier = options
            .idp_identifier
            .clone()
            .or_else(|| env::var(TEAM_IDP_IDENTIFIER_ENV).ok())
            .or_else(|| {
                current
                    .as_ref()
                    .and_then(|team| team.idp_identifier.clone())
            })
            .or_else(|| Some(DEFAULT_TEAM_IDP_IDENTIFIER.to_string()))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            graphql_endpoint,
            cognito_domain: normalize_cognito_domain(&cognito_domain),
            client_id,
            redirect_uri,
            scopes,
            idp_identifier,
        })
    }

    fn from_state(team: &TeamState) -> Self {
        Self {
            graphql_endpoint: team.graphql_endpoint.clone(),
            cognito_domain: team.cognito_domain.clone(),
            client_id: team.client_id.clone(),
            redirect_uri: team.redirect_uri.clone(),
            scopes: team.scopes.clone(),
            idp_identifier: team.idp_identifier.clone(),
        }
    }

    fn authorize_url(&self, challenge: &str, state_value: &str) -> String {
        let mut params = vec![
            ("response_type", "code".to_string()),
            ("client_id", self.client_id.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("scope", self.scopes.clone()),
            ("state", state_value.to_string()),
            ("code_challenge_method", "S256".to_string()),
            ("code_challenge", challenge.to_string()),
        ];
        if let Some(idp_identifier) = &self.idp_identifier {
            params.push(("idp_identifier", idp_identifier.clone()));
        }

        format!(
            "{}/oauth2/authorize?{}",
            self.cognito_domain,
            form_encode(&params)
        )
    }

    fn token_endpoint(&self) -> String {
        format!("{}/oauth2/token", self.cognito_domain)
    }
}

fn discover_team_app_config(app_url: &str) -> Result<TeamAppConfig> {
    let app_url = normalize_app_url(app_url);
    let html = fetch_text(&app_url, "TEAM app HTML")?;
    if let Some(config) = parse_team_app_config(&html)? {
        return Ok(config);
    }

    for script in extract_script_sources(&html) {
        let script_url = resolve_app_asset_url(&app_url, &script)
            .with_context(|| format!("failed to resolve TEAM app script {script}"))?;
        let javascript = fetch_text(&script_url, "TEAM app JavaScript")?;
        if let Some(config) = parse_team_app_config(&javascript)? {
            return Ok(config);
        }
    }

    bail!(
        "could not discover TEAM config from {app_url}; pass --endpoint, --domain, --client-id, and --redirect-uri explicitly"
    )
}

fn parse_team_app_config(source: &str) -> Result<Option<TeamAppConfig>> {
    let graphql_endpoint = match extract_js_string_field(source, "aws_appsync_graphqlEndpoint") {
        Some(value) => value,
        None => return Ok(None),
    };
    let client_id = extract_js_string_field(source, "aws_user_pools_web_client_id")
        .context("TEAM app config missing aws_user_pools_web_client_id")?;
    let oauth = oauth_config_window(source).unwrap_or(source);
    let cognito_domain =
        extract_js_string_field(oauth, "domain").context("TEAM app config missing oauth.domain")?;
    let redirect_uri = extract_js_string_field(oauth, "redirectSignIn")
        .context("TEAM app config missing oauth.redirectSignIn")?;
    let scopes = extract_js_scope_field(oauth);

    Ok(Some(TeamAppConfig {
        graphql_endpoint,
        cognito_domain,
        client_id,
        redirect_uri,
        scopes,
    }))
}

impl TeamIdentity {
    fn from_jwt(token: &str) -> Result<Self> {
        let payload = token
            .split('.')
            .nth(1)
            .context("TEAM auth token is not a JWT")?;
        let decoded = decode_base64_url(payload)?;
        let value: Value =
            serde_json::from_slice(&decoded).context("failed to decode TEAM auth token payload")?;
        let user_id = value
            .get("userId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .context("TEAM auth token does not contain userId")?;
        let group_ids = group_ids_from_claim(value.get("groupIds"))
            .context("TEAM auth token does not contain groupIds")?;
        if group_ids.is_empty() {
            bail!("TEAM auth token contains no groupIds");
        }

        Ok(Self { user_id, group_ids })
    }
}

impl TeamClient {
    fn new(config: TeamConfig) -> Self {
        Self { config }
    }

    fn resolve_request_target(
        &self,
        profile: &SsoProfile,
        identity: &TeamIdentity,
    ) -> Result<RequestTarget> {
        let data = self.call_graphql(
            GET_USER_POLICY,
            json!({
                "userId": identity.user_id,
                "groupIds": identity.group_ids,
            }),
        )?;
        let policy = data
            .get("getUserPolicy")
            .and_then(|value| value.get("policy"))
            .and_then(Value::as_array)
            .context("TEAM policy response did not include policy")?;

        for entitlement in policy {
            let Some(account) = entitlement
                .get("accounts")
                .and_then(Value::as_array)
                .and_then(|accounts| {
                    accounts.iter().find(|account| {
                        account.get("id").and_then(Value::as_str) == Some(&profile.account_id)
                    })
                })
            else {
                continue;
            };
            let Some(permission) = entitlement
                .get("permissions")
                .and_then(Value::as_array)
                .and_then(|permissions| {
                    permissions.iter().find(|permission| {
                        permission.get("name").and_then(Value::as_str) == Some(&profile.role_name)
                    })
                })
            else {
                continue;
            };

            return Ok(RequestTarget {
                account_id: profile.account_id.clone(),
                account_name: required_string(account, "name")?,
                role_name: profile.role_name.clone(),
                role_id: required_string(permission, "id")?,
                max_duration: entitlement
                    .get("duration")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }

        bail!(
            "TEAM policy does not list {} / {} as eligible for this user",
            profile.account_id,
            profile.role_name
        );
    }

    fn validate_request(&self, target: &RequestTarget, identity: &TeamIdentity) -> Result<()> {
        let data = self.call_graphql(
            VALIDATE_REQUEST,
            json!({
                "accountId": target.account_id,
                "roleId": target.role_id,
                "userId": identity.user_id,
                "groupIds": identity.group_ids,
            }),
        )?;
        let validation = data
            .get("validateRequest")
            .context("TEAM validateRequest response missing")?;
        if validation
            .get("valid")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let reason = validation
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown reason");
        bail!("TEAM denied this request: {reason}");
    }

    fn create_request(
        &self,
        target: &RequestTarget,
        input: &RequestInput,
    ) -> Result<ElevationOutcome> {
        let data = self.call_graphql(
            CREATE_REQUEST,
            json!({
                "input": {
                    "accountId": target.account_id,
                    "accountName": target.account_name,
                    "role": target.role_name,
                    "roleId": target.role_id,
                    "startTime": Utc::now().to_rfc3339(),
                    "duration": input.duration_hours,
                    "justification": input.justification,
                    "ticketNo": input.ticket_no,
                }
            }),
        )?;
        let request = data
            .get("createRequests")
            .context("TEAM createRequests response missing")?;
        Ok(ElevationOutcome::Submitted {
            id: required_string(request, "id")?,
            status: request
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending")
                .to_string(),
        })
    }

    fn call_graphql(&self, query: &str, variables: Value) -> Result<Value> {
        let payload = json!({
            "query": query,
            "variables": variables,
        })
        .to_string();

        let mut child = Command::new("curl")
            .args([
                "-sS",
                "-X",
                "POST",
                "-H",
                "content-type: application/json",
                "-H",
                &format!("Authorization: {}", self.config.token),
                "--data-binary",
                "@-",
                &self.config.endpoint,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to run curl for TEAM GraphQL request")?;

        child
            .stdin
            .as_mut()
            .context("failed to open curl stdin")?
            .write_all(payload.as_bytes())
            .context("failed to write TEAM GraphQL payload")?;

        let output = child
            .wait_with_output()
            .context("failed to wait for TEAM GraphQL request")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("TEAM GraphQL request failed: {stderr}");
        }

        let response: Value = serde_json::from_slice(&output.stdout)
            .context("TEAM GraphQL response was not valid JSON")?;
        if let Some(errors) = response.get("errors") {
            bail!("TEAM GraphQL returned errors: {errors}");
        }
        response
            .get("data")
            .cloned()
            .context("TEAM GraphQL response missing data")
    }
}

#[derive(Debug, Clone)]
struct RequestInput {
    duration_hours: String,
    ticket_no: String,
    justification: String,
}

fn collect_request_input(
    profile: &SsoProfile,
    target: &RequestTarget,
    options: &ElevationOptions,
) -> Result<RequestInput> {
    let default_duration = options
        .duration_hours
        .clone()
        .or_else(|| env::var(DEFAULT_DURATION_ENV).ok())
        .unwrap_or_else(|| "1".to_string());
    let duration_question = target
        .max_duration
        .as_deref()
        .map(|max_duration| format!("Duration hours (1-{max_duration}):"))
        .unwrap_or_else(|| "Duration hours:".to_string());
    let duration_hours = prompt_if_missing(
        options
            .duration_hours
            .clone()
            .or_else(|| env::var(DEFAULT_DURATION_ENV).ok()),
        &duration_question,
        Some(&default_duration),
    )?;
    validate_duration(&duration_hours, target.max_duration.as_deref())?;

    let ticket_no = prompt_if_missing(
        options
            .ticket_no
            .clone()
            .or_else(|| env::var(DEFAULT_TICKET_ENV).ok()),
        "Ticket no:",
        None,
    )?;
    validate_nonempty("ticket number", &ticket_no)?;

    let justification = prompt_if_missing(
        options
            .justification
            .clone()
            .or_else(|| env::var(DEFAULT_JUSTIFICATION_ENV).ok()),
        &format!("Justification for {}:", profile.name),
        None,
    )?;
    validate_nonempty("justification", &justification)?;

    Ok(RequestInput {
        duration_hours,
        ticket_no,
        justification,
    })
}

fn option_or_env_or_discovered_or_state(
    value: &Option<String>,
    env_name: &str,
    discovered_value: Option<&str>,
    state_value: Option<&str>,
) -> Option<String> {
    value
        .clone()
        .or_else(|| env::var(env_name).ok())
        .or_else(|| discovered_value.map(str::to_string))
        .or_else(|| state_value.map(str::to_string))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_cognito_domain(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

fn normalize_app_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

fn fetch_text(url: &str, label: &str) -> Result<String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", url])
        .output()
        .with_context(|| format!("failed to run curl for {label}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to fetch {label} from {url}: {stderr}");
    }

    String::from_utf8(output.stdout).with_context(|| format!("{label} was not valid UTF-8"))
}

fn extract_script_sources(html: &str) -> Vec<String> {
    let mut sources = Vec::new();
    let mut rest = html;
    while let Some(index) = rest.find("src=") {
        rest = &rest[index + "src=".len()..];
        let Some(quote) = rest.as_bytes().first().copied() else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        rest = &rest[1..];
        let Some(end) = rest.find(quote as char) else {
            break;
        };
        let value = &rest[..end];
        if value.contains(".js") {
            sources.push(value.to_string());
        }
        rest = &rest[end + 1..];
    }
    sources
}

fn resolve_app_asset_url(app_url: &str, asset: &str) -> Result<String> {
    let asset = asset.trim();
    if asset.starts_with("http://") || asset.starts_with("https://") {
        return Ok(asset.to_string());
    }

    let scheme_end = app_url.find("://").context("app URL is missing a scheme")?;
    let after_scheme = scheme_end + "://".len();
    let path_start = app_url[after_scheme..]
        .find('/')
        .map(|index| after_scheme + index);
    let origin = path_start.map(|index| &app_url[..index]).unwrap_or(app_url);

    if asset.starts_with("//") {
        return Ok(format!("{}:{asset}", &app_url[..scheme_end]));
    }
    if asset.starts_with('/') {
        return Ok(format!("{origin}{asset}"));
    }

    let base = if app_url.ends_with('/') {
        app_url.to_string()
    } else if let Some(index) = app_url.rfind('/') {
        if index >= after_scheme {
            app_url[..=index].to_string()
        } else {
            format!("{origin}/")
        }
    } else {
        format!("{origin}/")
    };
    Ok(format!("{base}{asset}"))
}

fn oauth_config_window(source: &str) -> Option<&str> {
    let config_start = source.find("aws_appsync_graphqlEndpoint")?;
    let start = config_start + source[config_start..].find("oauth")?;
    let end = (start + 1_500).min(source.len());
    Some(&source[start..end])
}

fn extract_js_string_field(source: &str, key: &str) -> Option<String> {
    let mut offset = 0;
    while let Some(index) = source[offset..].find(key) {
        let start = offset + index;
        let mut after_key = source[start + key.len()..].trim_start();
        if after_key.starts_with('"') || after_key.starts_with('\'') {
            after_key = after_key[1..].trim_start();
        }
        let Some(after_colon) = after_key.strip_prefix(':') else {
            offset = start + key.len();
            continue;
        };
        if let Some(value) = read_js_string(after_colon.trim_start()) {
            return Some(value);
        }
        offset = start + key.len();
    }
    None
}

fn extract_js_scope_field(source: &str) -> Option<String> {
    let index = source.find("scope")?;
    let mut after_key = source[index + "scope".len()..].trim_start();
    if after_key.starts_with('"') || after_key.starts_with('\'') {
        after_key = after_key[1..].trim_start();
    }
    let after_colon = after_key.strip_prefix(':')?.trim_start();
    if let Some(value) = read_js_string(after_colon) {
        return Some(value);
    }

    let array = after_colon.strip_prefix('[')?;
    let end = array.find(']')?;
    let mut scopes = Vec::new();
    let mut rest = &array[..end];
    while let Some(index) = rest.find(|char| char == '"' || char == '\'') {
        rest = &rest[index..];
        let quote = rest.as_bytes().first().copied()?;
        let mut value = String::new();
        let mut escaped = false;
        let mut consumed = None;
        for (offset, char) in rest[1..].char_indices() {
            if escaped {
                value.push(char);
                escaped = false;
                continue;
            }
            if char == '\\' {
                escaped = true;
                continue;
            }
            if char as u8 == quote {
                consumed = Some(offset + 2);
                break;
            }
            value.push(char);
        }
        scopes.push(value);
        rest = &rest[consumed?..];
    }
    if scopes.is_empty() {
        None
    } else {
        Some(scopes.join(" "))
    }
}

fn read_js_string(source: &str) -> Option<String> {
    let quote = source.as_bytes().first().copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }

    let mut output = String::new();
    let mut escaped = false;
    for char in source[1..].chars() {
        if escaped {
            output.push(char);
            escaped = false;
            continue;
        }
        if char == '\\' {
            escaped = true;
            continue;
        }
        if char as u8 == quote {
            return Some(output);
        }
        output.push(char);
    }
    None
}

fn code_verifier() -> String {
    (0..3)
        .map(|_| Uuid::new_v4().simple().to_string())
        .collect::<String>()
}

fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64_url_encode(&digest)
}

fn exchange_authorization_code(
    auth: &TeamAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<TokenResponse> {
    let fields = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", auth.client_id.clone()),
        ("code", code.to_string()),
        ("code_verifier", verifier.to_string()),
        ("redirect_uri", auth.redirect_uri.clone()),
    ];
    parse_token_response(post_form(&auth.token_endpoint(), &fields)?)
}

fn refresh_tokens(auth: &TeamAuthConfig, refresh_token: &str) -> Result<TokenResponse> {
    let fields = vec![
        ("grant_type", "refresh_token".to_string()),
        ("client_id", auth.client_id.clone()),
        ("refresh_token", refresh_token.to_string()),
    ];
    let mut response = parse_token_response(post_form(&auth.token_endpoint(), &fields)?)?;
    if response.refresh_token.is_none() {
        response.refresh_token = Some(refresh_token.to_string());
    }
    Ok(response)
}

fn post_form(endpoint: &str, fields: &[(&str, String)]) -> Result<Value> {
    let body = form_encode(fields);
    let mut child = Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            "-H",
            "content-type: application/x-www-form-urlencoded",
            "--data-binary",
            "@-",
            endpoint,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run curl for Cognito token request")?;

    child
        .stdin
        .as_mut()
        .context("failed to open curl stdin")?
        .write_all(body.as_bytes())
        .context("failed to write Cognito token request")?;

    let output = child
        .wait_with_output()
        .context("failed to wait for Cognito token request")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Cognito token request failed: {stderr}");
    }

    serde_json::from_slice(&output.stdout).context("Cognito token response was not valid JSON")
}

fn parse_token_response(response: Value) -> Result<TokenResponse> {
    if let Some(error) = response.get("error").and_then(Value::as_str) {
        let description = response
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("");
        bail!("Cognito token request failed: {error} {description}");
    }

    Ok(TokenResponse {
        id_token: required_string(&response, "id_token")?,
        access_token: required_string(&response, "access_token")?,
        refresh_token: response
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn persist_team_state(auth: &TeamAuthConfig, tokens: TokenResponse) -> Result<()> {
    state::set_team_state(TeamState {
        graphql_endpoint: auth.graphql_endpoint.clone(),
        cognito_domain: auth.cognito_domain.clone(),
        client_id: auth.client_id.clone(),
        redirect_uri: auth.redirect_uri.clone(),
        scopes: auth.scopes.clone(),
        idp_identifier: auth.idp_identifier.clone(),
        id_token: tokens.id_token,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        updated_at: Utc::now().to_rfc3339(),
    })
}

fn extract_authorization_code(input: &str, expected_state: Option<&str>) -> Result<String> {
    let query = query_part(input).context("redirected URL does not contain a query string")?;
    let params = parse_query(query);
    if let Some(error) = params.iter().find(|(key, _)| key == "error") {
        let description = params
            .iter()
            .find(|(key, _)| key == "error_description")
            .map(|(_, value)| value.as_str())
            .unwrap_or("");
        bail!("Cognito authorization failed: {} {}", error.1, description);
    }
    if let Some(expected_state) = expected_state {
        if let Some((_, state_value)) = params.iter().find(|(key, _)| key == "state") {
            if state_value != expected_state {
                bail!("redirect state did not match the login request");
            }
        }
    }
    params
        .into_iter()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value)
        .context("redirected URL did not contain an authorization code")
}

fn query_part(input: &str) -> Option<&str> {
    let (_, rest) = input.split_once('?')?;
    Some(rest.split('#').next().unwrap_or(rest))
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

fn token_is_fresh(token: &str) -> bool {
    token_exp(token)
        .map(|expires_at| expires_at > Utc::now().timestamp() + 60)
        .unwrap_or(false)
}

fn token_status(token: &str) -> Option<String> {
    let expires_at = token_exp(token)?;
    let remaining = expires_at - Utc::now().timestamp();
    if remaining <= 0 {
        return Some("expired".to_string());
    }
    Some(format!("valid for {}m", remaining / 60))
}

fn token_exp(token: &str) -> Option<i64> {
    let payload = token.split('.').nth(1)?;
    let decoded = decode_base64_url(payload).ok()?;
    let value = serde_json::from_slice::<Value>(&decoded).ok()?;
    value.get("exp").and_then(Value::as_i64)
}

fn open_browser(url: &str) {
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", url]).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    };
    if !matches!(status, Ok(status) if status.success()) {
        eprintln!("Could not open a browser automatically; use the URL above.");
    }
}

fn prompt_if_missing(
    value: Option<String>,
    question: &str,
    default: Option<&str>,
) -> Result<String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => prompt::text(question, default),
    }
}

fn validate_duration(duration: &str, max_duration: Option<&str>) -> Result<()> {
    let value = duration
        .parse::<u32>()
        .with_context(|| format!("duration must be a whole number of hours, got {duration}"))?;
    if value == 0 {
        bail!("duration must be at least 1 hour");
    }
    if let Some(max_duration) = max_duration.and_then(|max| max.parse::<u32>().ok()) {
        if value > max_duration {
            bail!("duration must be at most {max_duration} hours");
        }
    }
    Ok(())
}

fn validate_nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} is required for TEAM request submission");
    }
    Ok(())
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("TEAM response missing {field}"))
}

fn group_ids_from_claim(value: Option<&Value>) -> Option<Vec<String>> {
    match value? {
        Value::String(value) => Some(
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
        ),
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
        ),
        _ => None,
    }
}

fn decode_base64_url(value: &str) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u8;

    for byte in value.bytes().filter(|byte| *byte != b'=') {
        let sextet = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => bail!("TEAM auth token payload is not base64url"),
        };
        buffer = (buffer << 6) | u32::from(sextet);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    Ok(output)
}

fn base64_url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let second = bytes.get(index + 1).copied();
        let third = bytes.get(index + 2).copied();

        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(
            TABLE[(((first & 0b0000_0011) << 4) | (second.unwrap_or(0) >> 4)) as usize] as char,
        );
        if let Some(second) = second {
            output.push(
                TABLE[(((second & 0b0000_1111) << 2) | (third.unwrap_or(0) >> 6)) as usize] as char,
            );
        }
        if let Some(third) = third {
            output.push(TABLE[(third & 0b0011_1111) as usize] as char);
        }

        index += 3;
    }
    output
}

fn form_encode(fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn percent_decode(value: &str) -> String {
    let mut output = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    output.push(byte);
                    index += 3;
                } else {
                    output.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_jwt_identity_claims() {
        let token = "eyJhbGciOiJub25lIn0.eyJ1c2VySWQiOiJ1LTEiLCJncm91cElkcyI6ImctMSxnLTIifQ.";

        let identity = TeamIdentity::from_jwt(token).unwrap();

        assert_eq!(identity.user_id, "u-1");
        assert_eq!(identity.group_ids, vec!["g-1", "g-2"]);
    }

    #[test]
    fn parses_array_group_claims() {
        let groups = group_ids_from_claim(Some(&json!(["g-1", "g-2"]))).unwrap();

        assert_eq!(groups, vec!["g-1", "g-2"]);
    }

    #[test]
    fn validates_duration_against_policy_max() {
        assert!(validate_duration("1", Some("2")).is_ok());
        assert!(validate_duration("3", Some("2")).is_err());
        assert!(validate_duration("0", None).is_err());
    }

    #[test]
    fn builds_pkce_challenge() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

        assert_eq!(
            code_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn extracts_authorization_code_from_redirect() {
        let code = extract_authorization_code(
            "https://team.example/callback?code=abc%2B123&state=state-1",
            Some("state-1"),
        )
        .unwrap();

        assert_eq!(code, "abc+123");
    }

    #[test]
    fn rejects_redirect_state_mismatch() {
        assert!(extract_authorization_code(
            "https://team.example/callback?code=abc&state=bad",
            Some("good")
        )
        .is_err());
    }

    #[test]
    fn discovers_team_config_from_minified_amplify_bundle() {
        let javascript = r#"
            var Fo={aws_project_region:"us-east-1",
            aws_appsync_graphqlEndpoint:"https://example.appsync-api.us-east-1.amazonaws.com/graphql",
            aws_appsync_region:"us-east-1",
            aws_user_pools_web_client_id:"client-123",
            oauth:{domain:"team.auth.us-east-1.amazoncognito.com",
            scope:["phone","email","openid","profile","aws.cognito.signin.user.admin"],
            redirectSignIn:"https://team.example.com/",
            redirectSignOut:"https://team.example.com/",responseType:"code"}};
        "#;

        let config = parse_team_app_config(javascript).unwrap().unwrap();

        assert_eq!(
            config.graphql_endpoint,
            "https://example.appsync-api.us-east-1.amazonaws.com/graphql"
        );
        assert_eq!(
            config.cognito_domain,
            "team.auth.us-east-1.amazoncognito.com"
        );
        assert_eq!(config.client_id, "client-123");
        assert_eq!(config.redirect_uri, "https://team.example.com/");
        assert_eq!(
            config.scopes.unwrap(),
            "phone email openid profile aws.cognito.signin.user.admin"
        );
    }

    #[test]
    fn extracts_and_resolves_team_app_scripts() {
        let html = r#"
            <script defer="defer" src="/static/js/main.abc123.js"></script>
            <script src="runtime.js"></script>
        "#;

        assert_eq!(
            extract_script_sources(html),
            vec!["/static/js/main.abc123.js", "runtime.js"]
        );
        assert_eq!(
            resolve_app_asset_url("https://team.example.com/app/", "/static/js/main.js").unwrap(),
            "https://team.example.com/static/js/main.js"
        );
        assert_eq!(
            resolve_app_asset_url("https://team.example.com/app/index.html", "runtime.js").unwrap(),
            "https://team.example.com/app/runtime.js"
        );
    }
}
