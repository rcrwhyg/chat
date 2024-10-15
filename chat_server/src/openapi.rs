use axum::Router;
use chat_core::{Chat, ChatType, ChatUser, Message, User, Workspace};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::*;
use crate::{
    AppState, CreateChat, CreateMessage, CreateUser, ErrorOutput, ListMessages, SigninUser,
};

pub(crate) trait OpenApiRouter {
    fn openapi(self) -> Self;
}

#[derive(OpenApi)]
#[openapi(
    paths(
        signup_handler,
        signin_handler,
        list_chat_handler,
        create_chat_handler,
        get_chat_handler,
        update_chat_handler,
        list_message_handler,
        delete_chat_handler,
        send_message_handler,
        list_chat_users_handler,
    ),
    components  (
        schemas(Chat, ChatType, ChatUser, Message, User, Workspace, CreateChat, CreateMessage, CreateUser, ErrorOutput, ListMessages, SigninUser),
    ),
    modifiers(
        &SecurityAddon,
    ),
    tags  (
        (name = "chat", description = "Chat related operations"),
    ),
)]
pub(crate) struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(component) = openapi.components.as_mut() {
            component.add_security_scheme(
                "token",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        };
    }
}

impl OpenApiRouter for Router<AppState> {
    fn openapi(self) -> Self {
        self.merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi().clone()),
        )
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi().clone()))
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
    }
}
