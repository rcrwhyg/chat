use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use chat_core::{Chat, ChatType, Message};
use chat_server::AppState;
use futures::StreamExt as _;
use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::json;
use tokio::{net::TcpListener, time::sleep};

#[derive(Debug, Deserialize)]
struct AuthToken {
    token: String,
}

struct ChatServer {
    addr: SocketAddr,
    token: String,
    client: reqwest::Client,
}

struct NotifyServer;

const WILD_ADDR: &str = "127.0.0.1:0";

#[tokio::test]
async fn chat_server_should_work() -> Result<()> {
    let (tdb, state) = chat_server::AppState::try_new_for_test().await?;
    let chat_server = ChatServer::new(state).await?;
    let db_url = tdb.url();
    NotifyServer::new(&db_url, &chat_server.token).await?;
    let chat = chat_server.create_chat().await?;
    let _msg = chat_server.create_message(chat.id as u64).await?;
    sleep(Duration::from_secs(1)).await;
    Ok(())
}

impl NotifyServer {
    async fn new(db_url: &str, token: &str) -> Result<Self> {
        let mut config = notify_server::AppConfig::try_load()?;
        config.server.db_url = db_url.to_string();

        let app = notify_server::get_router(config).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let mut es = EventSource::get(format!("http://{}/events?access_token={}", addr, token));

        tokio::spawn(async move {
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => println!("Connection Open!"),
                    Ok(Event::Message(message)) => match message.event.as_str() {
                        "NewChat" => {
                            let chat = serde_json::from_str::<Chat>(&message.data).unwrap();
                            assert_eq!(chat.name.as_ref().unwrap(), "test");
                            assert_eq!(chat.members, vec![1, 2]);
                            assert_eq!(chat.r#type, ChatType::PrivateChannel);
                        }
                        "NewMessage" => {
                            let message = serde_json::from_str::<Message>(&message.data).unwrap();
                            assert_eq!(message.content, "hello");
                            assert_eq!(message.files.len(), 1);
                            assert_eq!(message.sender_id, 1);
                        }
                        _ => {
                            panic!("Unexpected event: {:?}", message);
                        }
                    },
                    Err(err) => {
                        println!("Error: {}", err);
                        es.close();
                    }
                }
            }
        });

        Ok(Self)
    }
}

impl ChatServer {
    async fn new(state: AppState) -> Result<Self> {
        let app = chat_server::get_router(state).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let client = reqwest::Client::new();

        let mut ret = Self {
            addr,
            token: "".to_string(),
            client,
        };

        ret.token = ret.signin().await?;
        Ok(ret)
    }

    async fn signin(&self) -> Result<String> {
        let resp = self
            .client
            .post(&format!("http://{}/api/signin", self.addr))
            .header("Content-Type", "application/json")
            .body(
                r#"
                {
                    "email": "tchen@acme.org",
                    "password": "123456"
                }
                "#,
            )
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::OK);

        let ret = resp.json::<AuthToken>().await?;
        Ok(ret.token)
    }

    async fn create_chat(&self) -> Result<Chat> {
        let resp = self
            .client
            .post(&format!("http://{}/api/chats", self.addr))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(
                r#"
                {
                    "name": "test",
                    "members": [1, 2],
                    "public": false
                }
                "#,
            )
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let chat = resp.json::<Chat>().await?;
        assert_eq!(chat.name.as_ref().unwrap(), "test");
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::PrivateChannel);

        Ok(chat)
    }

    async fn create_message(&self, chat_id: u64) -> Result<Message> {
        // upload file
        let data = include_bytes!("../Cargo.toml");
        let files = Part::bytes(data)
            .file_name("Cargo.toml")
            .mime_str("text/plain")?;
        let form = Form::new().part("file", files);

        let resp = self
            .client
            .post(&format!("http://{}/api/upload", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::OK);
        let ret = resp.json::<Vec<String>>().await?;

        let body = serde_json::to_string(&json!({
            "content": "hello",
            "files": ret
        }))?;
        let resp = self
            .client
            .post(&format!("http://{}/api/chats/{}", self.addr, chat_id))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(body)
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let msg = resp.json::<Message>().await?;
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.files, ret);
        assert_eq!(msg.sender_id, 1);
        assert_eq!(msg.chat_id, chat_id as i64);

        Ok(msg)
    }
}
