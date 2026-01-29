use anyhow::Result;
use clap::Subcommand;
use moltis_oauth::{CallbackServer, OAuthConfig, OAuthFlow, TokenStore};

#[derive(Subcommand)]
pub enum AuthAction {
    /// Log in to a provider via OAuth.
    Login {
        /// Provider name (e.g. "openai-codex").
        #[arg(long)]
        provider: String,
    },
    /// Show authentication status for all providers.
    Status,
    /// Log out from a provider.
    Logout {
        /// Provider name (e.g. "openai-codex").
        #[arg(long)]
        provider: String,
    },
}

pub async fn handle_auth(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { provider } => login(&provider).await,
        AuthAction::Status => status(),
        AuthAction::Logout { provider } => logout(&provider),
    }
}

fn oauth_config_for(provider: &str) -> Result<OAuthConfig> {
    match provider {
        "openai-codex" => Ok(OAuthConfig {
            client_id: "pdlLIX2Y72MIl2rhLhTE9VV9bN905kBh".to_string(),
            auth_url: "https://auth.openai.com/oauth/authorize".to_string(),
            token_url: "https://auth.openai.com/oauth/token".to_string(),
            redirect_uri: "http://127.0.0.1:1455/auth/callback".to_string(),
            scopes: vec![],
        }),
        _ => anyhow::bail!("unknown provider: {provider}"),
    }
}

async fn login(provider: &str) -> Result<()> {
    let config = oauth_config_for(provider)?;
    let flow = OAuthFlow::new(config);
    let req = flow.start();

    println!("Opening browser for authentication...");
    if open::that(&req.url).is_err() {
        println!("Could not open browser. Please visit:\n{}", req.url);
    }

    println!("Waiting for callback on http://127.0.0.1:1455/auth/callback ...");
    let code = CallbackServer::wait_for_code(1455, req.state).await?;

    println!("Exchanging code for tokens...");
    let tokens = flow.exchange(&code, &req.pkce.verifier).await?;

    let store = TokenStore::new();
    store.save(provider, &tokens)?;

    println!("Successfully logged in to {provider}");
    Ok(())
}

fn status() -> Result<()> {
    let store = TokenStore::new();
    let providers = store.list();
    if providers.is_empty() {
        println!("No authenticated providers.");
        return Ok(());
    }
    for provider in providers {
        if let Some(tokens) = store.load(&provider) {
            let expiry = tokens.expires_at.map_or("unknown".to_string(), |ts| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if ts > now {
                    let remaining = ts - now;
                    let hours = remaining / 3600;
                    let mins = (remaining % 3600) / 60;
                    format!("valid ({hours}h {mins}m remaining)")
                } else {
                    "expired".to_string()
                }
            });
            println!("{provider} [{expiry}]");
        }
    }
    Ok(())
}

fn logout(provider: &str) -> Result<()> {
    let store = TokenStore::new();
    store.delete(provider)?;
    println!("Logged out from {provider}");
    Ok(())
}
