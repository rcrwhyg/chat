use axum::{
    extract::{FromRequestParts, Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chat_core::User;

use crate::{AppError, AppState};

pub async fn verify_chat(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();
    let Path(chat_id) = Path::<u64>::from_request_parts(&mut parts, &state)
        .await
        .unwrap();

    let user = parts.extensions.get::<User>().unwrap();
    if !state
        .is_chat_member(chat_id, user.id as _)
        .await
        .unwrap_or_default()
    {
        let err = AppError::CreateMessageError(format!(
            "User {} is not a member of chat {}",
            user.id, chat_id
        ));
        return err.into_response();
    }

    let req = Request::from_parts(parts, body);

    next.run(req).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;
    use axum::{
        body::Body, http::StatusCode, middleware::from_fn_with_state, routing::get, Router,
    };
    use chat_core::middlewares::verify_token;
    use tower::ServiceExt;

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "OK")
    }

    #[tokio::test]
    async fn test_chat_middleware_should_work() -> Result<()> {
        let (_tdb, state) = AppState::try_new_for_test().await?;

        let user = state.find_user_by_id(1).await?.expect("user should exists");
        let token = state.ek.sign(user)?;

        let app = Router::new()
            .route("/chats/:id/messages", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_chat))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);

        // user in chat
        let req = Request::builder()
            .uri("/chats/1/messages")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.clone().oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);

        // user not in chat
        let req = Request::builder()
            .uri("/chats/5/messages")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}
