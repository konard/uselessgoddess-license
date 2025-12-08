use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use serde::Serialize;

use crate::model::*;
use crate::state::App;

#[derive(Serialize)]
pub struct Status {
    success: bool,
    msg: Option<String>,
}

pub async fn heartbeat(
    State(app): State<App>,
    Json(req): Json<HeartbeatReq>,
) -> (StatusCode, Json<Status>) {
    let now = Utc::now().naive_utc();

    if let Some(mut sessions) = app.sessions.get_mut(&req.key)
        && let Some(sess) = sessions.iter_mut().find(|sess| sess.machine_id == req.machine_id)
    {
        sess.last_seen = now;
        return (StatusCode::OK, Json(Status { success: true, msg: None }));
    }

    let license = sqlx::query_as!(License, "SELECT * FROM licenses WHERE key = ?", req.key)
        .fetch_optional(&app.db)
        .await;

    let license = match license {
        Ok(Some(l)) => l,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(Status { success: false, msg: Some("Invalid key".into()) }),
            );
        }
    };

    if license.is_blocked || license.expires_at < now {
        return (
            StatusCode::FORBIDDEN,
            Json(Status { success: false, msg: Some("Expired or blocked".into()) }),
        );
    }

    let mut entry = app.sessions.entry(req.key.clone()).or_insert_with(Vec::new);

    // GC dead sessions
    entry.retain(|s| (now - s.last_seen).num_seconds() < 60);

    if entry.len() >= 5 {
        return (
            StatusCode::CONFLICT,
            Json(Status { success: false, msg: Some("Limit reached".into()) }),
        );
    } else {
        entry.push(Session { machine_id: req.machine_id, last_seen: now });
    }

    (StatusCode::OK, Json(Status { success: true, msg: None }))
}
