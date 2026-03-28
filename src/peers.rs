use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use serde::Deserialize;

use crate::db::{self, PeerRow};
use crate::AppState;

#[derive(Deserialize, Default)]
pub struct PeersQuery {
    pub q: Option<String>,
    pub notice: Option<String>,
    pub err: Option<String>,
}

pub async fn page(
    State(st): State<AppState>,
    Query(query): Query<PeersQuery>,
) -> impl IntoResponse {
    let Some(ref pool) = st.db else {
        let body = r#"<h1>Peers</h1><p class="err">HBBS_DB_PATH is not configured.</p>
<p><a href="/dashboard">Dashboard</a></p>"#;
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            crate::html_page("Peers", body),
        );
    };

    let q = query.q.as_deref().unwrap_or("");
    let rows = match db::list_peers(pool, q).await {
        Ok(r) => r,
        Err(e) => {
            let body = format!(
                r#"<h1>Peers</h1><p class="err">Query failed: {}</p>
<p><a href="/dashboard">Dashboard</a></p>"#,
                esc(&e.to_string())
            );
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                crate::html_page("Peers", &body),
            );
        }
    };

    let mut body = String::from(
        r#"<h1>Peers</h1>
<div class="callout">
<p><strong>Online / last seen:</strong> Stock hbbs keeps “recently registered” only <em>in memory</em>
(about 30s window for online checks). SQLite stores <code>created_at</code>, <code>status</code>, <code>info</code> (JSON, usually last IP), and <code>note</code> — not a reliable “last seen offline” timestamp. Use <code>status</code> / <code>info</code> as hints only.</p>
</div>
"#,
    );

    if let Some(ref n) = query.notice {
        if !n.is_empty() {
            body.push_str(&format!(
                r#"<p class="notice">{}</p>"#,
                humanize_notice(n)
            ));
        }
    }
    if let Some(ref e) = query.err {
        if !e.is_empty() {
            body.push_str(&format!(r#"<p class="err">{}</p>"#, humanize_err(e)));
        }
    }

    body.push_str(r#"<form class="search" method="get" action="/peers"><label>Search <input type="search" name="q" placeholder="id, note, info…" value=""#);
    body.push_str(&esc_attr(q));
    body.push_str(r#""/></label> <button type="submit">Search</button></form>"#);
    body.push_str(&format!(
        r#"<p class="muted">{} row(s). <a href="/peers">Clear search</a> · <a href="/dashboard">Dashboard</a></p>"#,
        rows.len()
    ));

    body.push_str(
        r#"<div class="table-wrap"><table><thead><tr>
<th>ID</th><th>Connect</th><th>guid</th><th>uuid</th><th>pk (trunc.)</th><th>user</th>
<th>created_at</th><th>status</th><th>note</th><th>info</th><th>Rename ID</th><th>Delete</th>
</tr></thead><tbody>"#,
    );

    for row in &rows {
        body.push_str(&render_row(row, st.config.rustdesk_connect_rendezvous.as_deref()));
    }

    body.push_str("</tbody></table></div>");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        crate::html_page("Peers", &body),
    )
}

fn render_row(row: &PeerRow, rendezvous: Option<&str>) -> String {
    let guid_hex = hex::encode(&row.guid);
    let connect = connect_href(&row.id, rendezvous);
    let uuid_hex = hex::encode(&row.uuid);
    let pk_hex = hex::encode(&row.pk);
    let pk_short = if pk_hex.len() > 24 {
        format!("{}…", &pk_hex[..24])
    } else {
        pk_hex.clone()
    };
    let user_cell = match &row.user {
        Some(u) if !u.is_empty() => format!(r#"<code title="{}">{}…</code>"#, hex::encode(u), hex::encode(&u[..u.len().min(8)])),
        _ => r"&mdash;".to_string(),
    };
    let status_cell = row
        .status
        .map(|s| s.to_string())
        .unwrap_or_else(|| "&mdash;".to_string());
    let note_cell = row
        .note
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format!(r#"<span title="{}">{}</span>"#, esc_attr(s), esc(s)))
        .unwrap_or_else(|| "&mdash;".to_string());
    let info_esc = esc(&row.info);
    let info_short = if row.info.chars().count() > 120 {
        esc(&row.info.chars().take(120).collect::<String>()) + "…"
    } else {
        info_esc.clone()
    };

    format!(
        r#"<tr>
<td><strong>{}</strong></td>
<td><a class="connect" href="{}">Open in RustDesk</a><span class="muted small">Windows / Android if the app registered <code>rustdesk://</code></span></td>
<td><code class="blob">{}</code></td>
<td><code class="blob">{}</code></td>
<td><code class="blob" title="{}">{}</code></td>
<td>{}</td>
<td>{}</td>
<td>{}</td>
<td>{}</td>
<td><code class="info" title="{}">{}</code></td>
<td><form method="post" action="/peers/rename" class="inline">
<input type="hidden" name="guid_hex" value="{}"/>
<input type="text" name="new_id" required maxlength="100" placeholder="new id" aria-label="New ID for {}"/>
<button type="submit">Save</button>
</form></td>
<td><form method="post" action="/peers/delete" class="inline" onsubmit="return confirm('Delete this peer? This cannot be undone.');">
<input type="hidden" name="guid_hex" value="{}"/>
<button type="submit" class="danger">Delete</button>
</form></td>
</tr>"#,
        esc(&row.id),
        connect,
        guid_hex,
        uuid_hex,
        pk_hex,
        pk_short,
        user_cell,
        esc(&row.created_at),
        status_cell,
        note_cell,
        info_esc,
        info_short,
        guid_hex,
        esc_attr(&row.id),
        guid_hex,
    )
}

fn connect_href(id: &str, rendezvous: Option<&str>) -> String {
    let id_enc = urlencoding::encode(id);
    match rendezvous {
        Some(h) if !h.trim().is_empty() => {
            format!(
                "rustdesk://{}/r@{}",
                id_enc,
                urlencoding::encode(h.trim())
            )
        }
        _ => format!("rustdesk://{id_enc}"),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn esc_attr(s: &str) -> String {
    esc(s).replace('\'', "&#39;")
}

fn humanize_notice(raw: &str) -> String {
    esc(&raw.replace('+', " "))
}

fn humanize_err(raw: &str) -> String {
    let s = raw.replace('+', " ");
    let text = match s.as_str() {
        "no_db" => "Database is not configured.",
        "invalid_guid" => "Invalid row identifier.",
        "no_such_peer" => "No matching peer (it may have been removed already).",
        "delete_failed" => {
            "Delete failed (see server logs; hbbs may be locking the database — retry or stop hbbs briefly)."
        }
        "rename_failed" => "Rename failed (see server logs).",
        "invalid_new_id" => "New ID must be 1–100 characters after trimming spaces.",
        other => other,
    };
    esc(text)
}

#[derive(Deserialize)]
pub struct GuidForm {
    pub guid_hex: String,
}

#[derive(Deserialize)]
pub struct RenameForm {
    pub guid_hex: String,
    pub new_id: String,
}

pub async fn post_delete(
    State(st): State<AppState>,
    Form(form): Form<GuidForm>,
) -> impl IntoResponse {
    let Some(ref pool) = st.db else {
        return Redirect::to("/peers?err=no_db").into_response();
    };
    let Ok(guid) = hex::decode(form.guid_hex.trim()) else {
        return Redirect::to("/peers?err=invalid_guid").into_response();
    };
    match db::delete_peer_by_guid(pool, &guid).await {
        Ok(0) => Redirect::to("/peers?err=no_such_peer").into_response(),
        Ok(_) => Redirect::to("/peers?notice=Peer+deleted").into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "delete_peer");
            Redirect::to("/peers?err=delete_failed").into_response()
        }
    }
}

pub async fn post_rename(
    State(st): State<AppState>,
    Form(form): Form<RenameForm>,
) -> impl IntoResponse {
    let Some(ref pool) = st.db else {
        return Redirect::to("/peers?err=no_db").into_response();
    };
    let Ok(guid) = hex::decode(form.guid_hex.trim()) else {
        return Redirect::to("/peers?err=invalid_guid").into_response();
    };
    let new_id = form.new_id.trim();
    if new_id.is_empty() || new_id.len() > 100 {
        return Redirect::to("/peers?err=invalid_new_id").into_response();
    }
    match db::update_peer_id(pool, &guid, new_id).await {
        Ok(()) => Redirect::to("/peers?notice=ID+updated").into_response(),
        Err(db::UpdateIdError::NotFound) => Redirect::to("/peers?err=no_such_peer").into_response(),
        Err(db::UpdateIdError::DuplicateId) => {
            Redirect::to("/peers?err=That+ID+is+already+in+use").into_response()
        }
        Err(db::UpdateIdError::Sql(e)) => {
            tracing::warn!(error = %e, "update_peer_id");
            Redirect::to("/peers?err=rename_failed").into_response()
        }
    }
}
</think>
Fixing `render_row`: correcting the `info` preview (remove erroneous `PeerRow::chars`).

<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>
StrReplace