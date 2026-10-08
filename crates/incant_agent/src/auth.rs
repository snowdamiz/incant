//! Public-client Sign in with ChatGPT, per the official 2026-10-08 contract.
//! OAuth records live in the OS keychain. No Codex or Claude sessions are read.
use crate::{AgentError, credentials::CredentialStore};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use url::Url;
use zeroize::ZeroizeOnDrop;

const AUTHORIZE: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const RESOURCE: &str = "https://api.openai.com/v1";
#[derive(Serialize, Deserialize, ZeroizeOnDrop)]
pub struct OAuthCredential {
    pub client_id: String,
    pub subject: String,
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub scope: String,
    pub expires_at: u64,
}
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    scope: Option<String>,
    expires_in: u64,
    token_type: String,
}
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
    revocation_endpoint: String,
}
#[derive(Clone, Deserialize)]
struct Claims {
    sub: String,
    nonce: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountMetadata {
    pub id: String,
    pub client_id: String,
    pub subject: String,
}
/// An attempt owns the callback listener, state, nonce and PKCE verifier together.
pub struct LoginAttempt {
    listener: TcpListener,
    state: String,
    nonce: String,
    verifier: String,
    redirect: String,
    client_id: String,
    url: Url,
}
fn failure(message: &str) -> AgentError {
    AgentError::Provider(message.into())
}
fn client() -> Result<Client, AgentError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| failure("auth HTTP client failed"))
}
fn random_secret() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl LoginAttempt {
    pub fn new(host_id: &str, returning_client_id: Option<&str>) -> Result<Self, AgentError> {
        if host_id.is_empty() {
            return Err(failure("a persistent host ID is required"));
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|_| failure("could not bind loopback callback"))?;
        listener
            .set_nonblocking(true)
            .map_err(|_| failure("callback listener initialization failed"))?;
        let redirect = format!(
            "http://127.0.0.1:{}/auth/callback",
            listener
                .local_addr()
                .map_err(|_| failure("callback listener unavailable"))?
                .port()
        );
        let state = random_secret();
        let nonce = random_secret();
        let verifier = random_secret();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let client_id = returning_client_id
            .unwrap_or("dynamic_agent_client")
            .to_string();
        let mut url = Url::parse(AUTHORIZE).expect("constant auth URL");
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("client_id", &client_id)
                .append_pair("ext_agent_host_id", host_id)
                .append_pair("response_type", "code")
                .append_pair("redirect_uri", &redirect)
                .append_pair(
                    "scope",
                    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
                )
                .append_pair("resource", RESOURCE)
                .append_pair("state", &state)
                .append_pair("nonce", &nonce)
                .append_pair("code_challenge_method", "S256")
                .append_pair("code_challenge", &challenge);
            if returning_client_id.is_none() {
                query.append_pair("agent_name_hint", "Incant");
            }
        }
        Ok(Self {
            listener,
            state,
            nonce,
            verifier,
            redirect,
            client_id,
            url,
        })
    }
    pub fn authorization_url(&self) -> &str {
        self.url.as_str()
    }
    pub fn finish(
        self,
        expected_subject: Option<&str>,
        timeout: Duration,
    ) -> Result<AccountMetadata, AgentError> {
        let deadline = Instant::now() + timeout;
        let (code, issued_client) = loop {
            if Instant::now() >= deadline {
                return Err(failure("sign-in timed out"));
            }
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .map_err(|_| failure("callback read timeout failed"))?;
                    let mut bytes = Vec::new();
                    let mut chunk = [0u8; 1024];
                    loop {
                        let size = stream
                            .read(&mut chunk)
                            .map_err(|_| failure("callback read failed"))?;
                        if size == 0 {
                            break;
                        }
                        bytes.extend_from_slice(&chunk[..size]);
                        if bytes.len() > 8192 {
                            return Err(failure("oversized callback"));
                        }
                        if bytes.windows(4).any(|x| x == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let line = std::str::from_utf8(&bytes)
                        .map_err(|_| failure("malformed callback"))?
                        .lines()
                        .next()
                        .unwrap_or("");
                    let mut parts = line.split_whitespace();
                    let method = parts.next();
                    let target = parts.next().unwrap_or("");
                    if method != Some("GET") || !target.starts_with("/auth/callback?") {
                        let _=stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        continue;
                    }
                    let result = self.validate_callback(target);
                    let body = if result.is_ok() {
                        "Authorization received. Return to Incant to finish validation."
                    } else {
                        "Authorization rejected. Return to Incant and retry sign-in."
                    };
                    let status = if result.is_ok() {
                        "200 OK"
                    } else {
                        "400 Bad Request"
                    };
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    break result?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                Err(_) => return Err(failure("callback listener failed")),
            }
        };
        let client = client()?;
        let response = client
            .post(TOKEN)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", issued_client.as_str()),
                ("code", code.as_str()),
                ("code_verifier", self.verifier.as_str()),
                ("redirect_uri", self.redirect.as_str()),
                ("resource", RESOURCE),
            ])
            .send()
            .map_err(|_| failure("OAuth token exchange failed"))?;
        if !response.status().is_success() {
            return Err(failure("OAuth code rejected; start a fresh sign-in"));
        }
        let token: TokenResponse = response
            .json()
            .map_err(|_| failure("invalid OAuth token response"))?;
        if token.token_type.to_lowercase() != "bearer" {
            return Err(failure("unsupported OAuth token type"));
        }
        let id_token = token.id_token.ok_or_else(|| failure("missing ID token"))?;
        let claims = validate_identity(&client, &id_token, &issued_client, Some(&self.nonce))?;
        if expected_subject.is_some_and(|s| s != claims.sub) {
            return Err(failure("returning sign-in account mismatch"));
        }
        let scope = token
            .scope
            .ok_or_else(|| failure("missing granted scopes"))?;
        require_inference_scope(&scope)?;
        let record = OAuthCredential {
            client_id: issued_client.clone(),
            subject: claims.sub.clone(),
            access_token: token.access_token,
            refresh_token: token
                .refresh_token
                .ok_or_else(|| failure("missing renewable session"))?,
            id_token,
            scope,
            expires_at: now().saturating_add(token.expires_in),
        };
        let id = format!(
            "oauth-{:x}",
            Sha256::digest(format!("{}:{}", issued_client, claims.sub))
        );
        CredentialStore::save(
            &id,
            &serde_json::to_string(&record).map_err(|_| failure("credential encoding failed"))?,
        )?;
        Ok(AccountMetadata {
            id,
            client_id: issued_client,
            subject: claims.sub,
        })
    }
    fn validate_callback(&self, target: &str) -> Result<(String, String), AgentError> {
        let url = Url::parse(&format!("http://127.0.0.1{target}"))
            .map_err(|_| failure("malformed callback URL"))?;
        if url.path() != "/auth/callback" {
            return Err(failure("unexpected callback path"));
        }
        let mut params = std::collections::BTreeMap::new();
        for (key, value) in url.query_pairs() {
            if params
                .insert(key.into_owned(), value.into_owned())
                .is_some()
            {
                return Err(failure("duplicate callback parameter"));
            }
        }
        if params.get("state") != Some(&self.state) {
            return Err(failure("OAuth state mismatch"));
        }
        if params.contains_key("error") {
            return Err(failure("sign-in declined or unavailable"));
        }
        let issued = params
            .get("client_id")
            .cloned()
            .unwrap_or_else(|| self.client_id.clone());
        if issued == "dynamic_agent_client" || issued.trim().is_empty() {
            return Err(failure("registration did not issue a client ID"));
        }
        if self.client_id != "dynamic_agent_client" && issued != self.client_id {
            return Err(failure("OAuth client identity mismatch"));
        }
        let code = params
            .get("code")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| failure("callback has no code"))?;
        Ok((code.clone(), issued))
    }
}
fn discovery(client: &Client) -> Result<Discovery, AgentError> {
    let result: Discovery = client
        .get("https://auth.openai.com/.well-known/openid-configuration")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| failure("OpenAI discovery failed"))?;
    if result.issuer != "https://auth.openai.com" {
        return Err(failure("unexpected identity issuer"));
    }
    for endpoint in [&result.jwks_uri, &result.revocation_endpoint] {
        let url = Url::parse(endpoint).map_err(|_| failure("invalid identity endpoint"))?;
        if url.scheme() != "https" || url.host_str() != Some("auth.openai.com") {
            return Err(failure("untrusted identity endpoint"));
        }
    }
    Ok(result)
}
fn validate_identity(
    client: &Client,
    token: &str,
    client_id: &str,
    nonce: Option<&str>,
) -> Result<Claims, AgentError> {
    let discovery = discovery(client)?;
    let keys: JwkSet = client
        .get(discovery.jwks_uri)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| failure("identity signing keys unavailable"))?;
    let header = decode_header(token).map_err(|_| failure("invalid ID token header"))?;
    if header.alg != Algorithm::RS256 {
        return Err(failure("unexpected ID token algorithm"));
    }
    let key = keys
        .find(
            header
                .kid
                .as_deref()
                .ok_or_else(|| failure("missing signing key ID"))?,
        )
        .ok_or_else(|| failure("unknown signing key"))?;
    let key = DecodingKey::from_jwk(key).map_err(|_| failure("invalid signing key"))?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["https://auth.openai.com"]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.leeway = 30;
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| failure("ID token signature, issuer, audience or expiration invalid"))?
        .claims;
    if claims.sub.is_empty() || nonce.is_some_and(|n| claims.nonce.as_deref() != Some(n)) {
        return Err(failure("ID token identity or nonce mismatch"));
    }
    Ok(claims)
}
fn require_inference_scope(scope: &str) -> Result<(), AgentError> {
    if !scope
        .split_whitespace()
        .any(|s| s == "chatgpt.tokens.use.direct")
    {
        return Err(failure("ChatGPT plan usage was not authorized"));
    }
    Ok(())
}
/// Caller serializes this operation per account (the headless CLI holds an OS lock).
pub fn access_token(account: &AccountMetadata) -> Result<zeroize::Zeroizing<String>, AgentError> {
    let saved = CredentialStore::load(&account.id)?;
    let mut record: OAuthCredential =
        serde_json::from_str(&saved).map_err(|_| failure("invalid keychain credential record"))?;
    if record.client_id != account.client_id || record.subject != account.subject {
        return Err(failure("stored account identity mismatch"));
    }
    require_inference_scope(&record.scope)?;
    if record.expires_at <= now().saturating_add(60) {
        let client = client()?;
        let response = client
            .post(TOKEN)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", record.client_id.as_str()),
                ("refresh_token", record.refresh_token.as_str()),
                ("resource", RESOURCE),
            ])
            .send()
            .map_err(|_| failure("token refresh unavailable"))?;
        if !response.status().is_success() {
            return Err(failure("token refresh rejected; reauthorize this account"));
        }
        let refreshed: TokenResponse = response
            .json()
            .map_err(|_| failure("invalid refresh response"))?;
        if refreshed.token_type.to_lowercase() != "bearer" {
            return Err(failure("unsupported refreshed token type"));
        }
        if let Some(id) = refreshed.id_token {
            let claims = validate_identity(&client, &id, &record.client_id, None)?;
            if claims.sub != record.subject {
                return Err(failure("refreshed identity mismatch"));
            }
            record.id_token = id;
        }
        if let Some(scope) = refreshed.scope {
            require_inference_scope(&scope)?;
            record.scope = scope;
        }
        record.access_token = refreshed.access_token;
        if let Some(refresh) = refreshed.refresh_token {
            record.refresh_token = refresh;
        }
        record.expires_at = now().saturating_add(refreshed.expires_in);
        CredentialStore::save(
            &account.id,
            &serde_json::to_string(&record).map_err(|_| failure("credential encoding failed"))?,
        )?;
    }
    Ok(zeroize::Zeroizing::new(record.access_token.clone()))
}
/// Local deletion happens even if remote revocation cannot be confirmed.
pub fn disconnect(account: &AccountMetadata) -> Result<bool, AgentError> {
    let secret = CredentialStore::load(&account.id)?;
    let record: OAuthCredential =
        serde_json::from_str(&secret).map_err(|_| failure("invalid keychain credential record"))?;
    let revoked = (|| {
        let client = client()?;
        let endpoint = discovery(&client)?.revocation_endpoint;
        for attempt in 0..3 {
            let response = client
                .post(&endpoint)
                .form(&[
                    ("token", record.refresh_token.as_str()),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", record.client_id.as_str()),
                ])
                .send();
            if response.as_ref().is_ok_and(|r| r.status().is_success()) {
                return Ok::<_, AgentError>(true);
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_secs(1 << attempt));
            }
        }
        Ok(false)
    })()
    .unwrap_or(false);
    CredentialStore::delete(&account.id)?;
    Ok(revoked)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_and_pkce_callback_validation() {
        let a = LoginAttempt::new("urn:uuid:test", None).unwrap();
        let url = Url::parse(a.authorization_url()).unwrap();
        let pairs: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
        assert_eq!(
            pairs["code_challenge"],
            URL_SAFE_NO_PAD.encode(Sha256::digest(a.verifier.as_bytes()))
        );
        assert_eq!(pairs["agent_name_hint"], "Incant");
        assert!(
            a.validate_callback("/auth/callback?state=wrong&code=x&client_id=oaiapp_x")
                .is_err()
        );
        assert!(
            a.validate_callback(&format!("/auth/callback?state={}&code=x", a.state))
                .is_err()
        );
        assert!(
            a.validate_callback(&format!(
                "/auth/callback?state={}&error=access_denied",
                a.state
            ))
            .is_err()
        );
        assert!(
            a.validate_callback(&format!(
                "/auth/callback?state={}&code=x&code=y&client_id=oaiapp_x",
                a.state
            ))
            .is_err()
        );
        assert!(
            a.validate_callback(&format!(
                "/auth/callback?state={}&code=x&client_id=oaiapp_x",
                a.state
            ))
            .is_ok()
        );
    }
    #[test]
    fn returning_registration_cannot_swap_client() {
        let a = LoginAttempt::new("host", Some("oaiapp_original")).unwrap();
        assert!(
            a.validate_callback(&format!(
                "/auth/callback?state={}&code=x&client_id=oaiapp_other",
                a.state
            ))
            .is_err()
        );
        assert_eq!(
            a.validate_callback(&format!("/auth/callback?state={}&code=x", a.state))
                .unwrap()
                .1,
            "oaiapp_original"
        );
    }
}
