mod auth;
mod config;
mod db;
mod peers;

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::{from_fn, Next},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum::http::{header, StatusCode};
use config::Config;
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Option<SqlitePool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustdesk_server_admin=info".into()),
        )
        .init();

    let config = Arc::new(Config::from_env().map_err(|e| {
        eprintln!("{e}");
        e
    })?);

    let db = if let Some(ref p) = config.hbbs_db_path {
        Some(
            db::open_pool(p)
                .await
                .map_err(|e| format!("HBBS_DB_PATH ({p}): {e}"))?,
        )
    } else {
        None
    };

    tracing::info!("listening on http://{}", config.listen_addr);

    let state = AppState {
        config: config.clone(),
        db,
    };

    let cfg_for_guard = state.config.clone();
    let protected = Router::new()
        .route("/dashboard", get(dashboard))
        .route("/peers", get(peers::page))
        .route("/peers/delete", post(peers::post_delete))
        .route("/peers/rename", post(peers::post_rename))
        .route_layer(from_fn(move |req: Request, next: Next| {
            let cfg = cfg_for_guard.clone();
            async move {
                if !auth::verify_session_token(
                    &cfg,
                    &auth::session_cookie_value(req.headers()).unwrap_or_default(),
                ) {
                    return Redirect::temporary("/login").into_response();
                }
                next.run(req).await
            }
        }));

    let app = Router::new()
        .route("/", get(root))
        .route("/login", get(login_page).post(login_post))
        .route("/logout", post(logout_post))
        .merge(protected)
        .with_state(state);

    let listener = TcpListener::bind(&config.listen_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> impl IntoResponse {
    Redirect::temporary("/dashboard")
}

pub fn html_page(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{title}</title>
  <style>
    body {{ font-family: system-ui, sans-serif; max-width: 96rem; margin: 1.5rem auto; padding: 0 1rem; }}
    code {{ background: #f4f4f4; padding: 0.1em 0.35em; border-radius: 4px; }}
    .err {{ color: #b00020; }}
    .notice {{ color: #0a6629; }}
    .muted {{ color: #555; font-size: 0.9rem; }}
    .small {{ display: block; font-size: 0.75rem; margin-top: 0.25rem; }}
    input[type=password], input[type=text], input[type=search] {{ padding: 0.35rem 0.5rem; }}
    button {{ padding: 0.35rem 0.75rem; margin-top: 0.25rem; cursor: pointer; }}
    button.danger {{ background: #c62828; color: #fff; border: none; border-radius: 4px; }}
    .callout {{ background: #f8f4e8; border: 1px solid #e6dcc4; padding: 0.75rem 1rem; border-radius: 6px; margin: 1rem 0; }}
    .search {{ margin: 1rem 0; }}
    .table-wrap {{ overflow-x: auto; margin-top: 1rem; }}
    table {{ border-collapse: collapse; width: 100%; font-size: 0.85rem; }}
    th, td {{ border: 1px solid #ddd; padding: 0.35rem 0.5rem; vertical-align: top; }}
    th {{ background: #f0f0f0; text-align: left; }}
    code.blob {{ font-size: 0.72rem; word-break: break-all; }}
    code.info {{ font-size: 0.72rem; white-space: pre-wrap; word-break: break-word; }}
    form.inline {{ display: flex; flex-wrap: wrap; gap: 0.35rem; align-items: center; margin: 0; }}
    a.connect {{ font-weight: 600; }}
  </style>
</head>
<body>
{body}
</body>
</html>"#
    )
}

async fn login_page() -> impl IntoResponse {
    let body = r#"<h1>RustDesk server admin</h1>
<p class="muted">Sign in with <code>ADMIN_PASSWORD</code>.</p>
<form method="post" action="/login" autocomplete="current-password">
  <label for="password">Password</label><br/>
  <input id="password" type="password" name="password" required autofocus/>
  <div><button type="submit">Sign in</button></div>
</form>"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html_page("Admin login", body),
    )
}

#[derive(serde::Deserialize)]
struct LoginForm {
    password: String,
}

async fn login_post(
    State(st): State<AppState>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let ok = ct_eq_password(&form.password, &st.config.admin_password);
    if !ok {
        let body = r#"<h1>RustDesk server admin</h1>
<p class="err">Invalid password.</p>
<p><a href="/login">Try again</a></p>"#;
        return (
            StatusCode::UNAUTHORIZED,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html_page("Admin login", body),
        )
            .into_response();
    }
    let Ok(cookie) = auth::set_session_cookie(&st.config) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "failed to create session",
        )
            .into_response();
    };
    let mut res = Redirect::to("/dashboard").into_response();
    res.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    res
}

async fn logout_post() -> impl IntoResponse {
    let mut res = Redirect::to("/login").into_response();
    res.headers_mut().insert(
        header::SET_COOKIE,
        auth::clear_session_cookie_header_value()
            .parse()
            .unwrap(),
    );
    res
}

async fn dashboard(State(st): State<AppState>) -> impl IntoResponse {
    let (peer_line, db_note) = match &st.db {
        None => (
            "<p><em>HBBS_DB_PATH is not set.</em> Peer count is unavailable.</p>".to_string(),
            String::new(),
        ),
        Some(pool) => match db::peer_count(pool).await {
            Ok(n) => (
                format!("<p>Registered peers (hbbs DB): <strong>{n}</strong></p>"),
                format!(
                    "<p class=\"muted\">SQLite <code>{}</code> (read/write for peer management).</p>",
                    esc(st.config.hbbs_db_path.as_deref().unwrap_or(""))
                ),
            ),
            Err(e) => (
                format!(
                    "<p class=\"err\">Could not read hbbs database: {}</p>",
                    esc(&e.to_string())
                ),
                String::new(),
            ),
        },
    };

    let body = format!(
        r#"<h1>Dashboard</h1>
{peer_line}
{db_note}
<p><a href="/peers">Peer list</a> — search, connect via <code>rustdesk://</code>, rename ID, delete</p>
<p><a href="/logout">Log out</a> (POST form below for browsers without JS)</p>
<form method="post" action="/logout"><button type="submit">Log out</button></form>"#
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html_page("RustDesk admin", &body),
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn ct_eq_password(got: &str, expected: &str) -> bool {
    if got.len() != expected.len() {
        return false;
    }
    bool::from(got.as_bytes().ct_eq(expected.as_bytes()))
}
