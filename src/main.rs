use askama::Template;
use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};

use serde::{Deserialize, Serialize};
use std::{io, net::SocketAddr};
use tokio::{
    join,
    sync::{broadcast, mpsc},
};
use tower_http::services::ServeDir;
use tracing::info;

use crate::{
    main_room::{ChatRoom, ChatRoomAction, Message},
    messages::initialise_chat_websocket_router,
};
mod main_room;
mod messages;
#[derive(Template)]
#[template(path = "index.html")]
struct IndexHTMLTemplate;

#[derive(Debug, Deserialize, Serialize)]
enum MessageCommands {
    Send { content: String },
    Ack,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct MessageAddedEvent {
    content: String,
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("starting app");
    let (send_channels, recieve_channel) = mpsc::channel::<ChatRoomAction>(100);
    let (tx, _) = broadcast::channel::<Message>(100);

    let mut chat_room = ChatRoom::new(tx.clone(), recieve_channel);

    let dir = ServeDir::new("assets");
    let msgs = initialise_chat_websocket_router(send_channels, tx);
    let app = Router::new()
        .merge(msgs)
        .route("/", get(root))
        .nest_service("/assets", get_service(dir).handle_error(handle_error));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    tracing::info!("listening on {}", addr);
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let _ = join!(server, chat_room.listen());
}

async fn root() -> Response<String> {
    let rendered_html = IndexHTMLTemplate.render().unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(rendered_html)
        .unwrap()
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
