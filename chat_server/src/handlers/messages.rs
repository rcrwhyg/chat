use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Extension, Json,
};
use tokio::fs;
use tracing::{info, warn};

use crate::{AppError, AppState, ChatFile, User};

pub(crate) async fn send_message_handler() -> impl IntoResponse {
    "send message"
}

pub(crate) async fn list_message_handler() -> impl IntoResponse {
    "list message"
}

pub(crate) async fn upload_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let ws_id = user.ws_id as u64;
    let base_dir = state.config.server.base_url.join(ws_id.to_string());
    let mut files = vec![];

    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().map(|name| name.to_string());
        let (Some(filename), Ok(data)) = (filename, field.bytes().await) else {
            warn!("Failed to read multipart field");
            continue;
        };

        let file = ChatFile::new(&filename, &data);
        let path = file.path(&base_dir);
        if path.exists() {
            info!("File {} already exists: {:?}", filename, path);
        } else {
            fs::create_dir_all(path.parent().expect("File path parent should exists")).await?;
            fs::write(path, data).await?;
        }

        files.push(file.url(ws_id));
    }

    Ok(Json(files))
}
