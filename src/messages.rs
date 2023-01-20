use crate::main_room::{ChatRoomAction, Message as ChatMessage};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use ciborium::de::from_reader;
use ciborium::ser::into_writer;
use serde::Deserialize;
use std::io::{BufWriter, Cursor};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};

#[derive(Debug, Clone)]
struct WebsocketChannels {
    send_channel: mpsc::Sender<ChatRoomAction>,
    subscription_channel: broadcast::Sender<ChatMessage>,
}

pub fn initialise_chat_websocket_router(
    send_channel: mpsc::Sender<ChatRoomAction>,
    subscription_channel: broadcast::Sender<ChatMessage>,
) -> Router {
    let channels = WebsocketChannels {
        send_channel,
        subscription_channel,
    };
    Router::new()
        .route("/chat", get(websocket_chat_protocol_upgrader))
        .with_state(channels)
}

async fn websocket_chat_protocol_upgrader(
    ws_u: WebSocketUpgrade,
    State(channels): State<WebsocketChannels>,
) -> Response {
    ws_u.on_upgrade(|ws| async move {
        handle_upgraded_websocket(
            ws,
            channels.send_channel,
            channels.subscription_channel.subscribe(),
        )
        .await;
    })
}
#[derive(Deserialize, Debug)]
enum ChatRequest {
    SendMessage { content: String, author: String },
    GetMessages,
}
async fn handle_upgraded_websocket(
    mut socket: WebSocket,
    message_send_channel: mpsc::Sender<ChatRoomAction>,
    mut subscription_channel: broadcast::Receiver<ChatMessage>,
) {
    'main_loop: loop {
        select! {
            end_of_websocket_connection_or_result_of_message_recieval = socket.recv() => {
                let result_of_message_recieval = if let Some(result_of_message_recieval) = end_of_websocket_connection_or_result_of_message_recieval {
                    result_of_message_recieval
                } else {
                    break 'main_loop;
                };
                let message = if let Ok(msg) = result_of_message_recieval {
                    let data = msg.into_data();
                    let cursor = Cursor::new(data);
                    if let Ok(message) = from_reader::<ChatRequest, Cursor<Vec<u8>>>(cursor) {
                        message
                    } else {
                        break 'main_loop;
                    }
                } else {
                    break 'main_loop;
                };

                match message {
                    ChatRequest::SendMessage { content, author } => {
                        if handle_sending_a_message(author, content, &message_send_channel, &mut socket).await.is_err()
                        {
                            break 'main_loop;
                        }
                    }
                    ChatRequest::GetMessages => {
                        if handle_message_get_command(&message_send_channel, &mut socket).await.is_err()
                        {
                            break 'main_loop;
                        }
                    }
                }
            }
            new_message_or_error = subscription_channel.recv() => {
                if let Ok(new_message) = new_message_or_error {
                    let mut writer = BufWriter::new(Vec::new());
                    if  into_writer(&new_message, writer.get_mut()).is_err() {
                        break 'main_loop;
                    }
                    if socket
                        .send(Message::Binary(writer.into_inner().unwrap()))
                        .await
                        .is_err()
                    {
                        break 'main_loop;
                    }
                }else {
                    break 'main_loop;
                }
            }
        }
    }
}

async fn handle_message_get_command(
    msg_send: &mpsc::Sender<ChatRoomAction>,
    socket: &mut WebSocket,
) -> Result<(), ()> {
    let (send, recieve) = oneshot::channel();
    if msg_send
        .send(ChatRoomAction::GetMessages {
            recieve_channel: send,
        })
        .await
        .is_err()
    {
        return Err(());
    }
    let response = if let Ok(response) = recieve.await {
        response
    } else {
        return Err(());
    };
    let mut writer = BufWriter::new(Vec::new());
    if into_writer(&response, writer.get_mut()).is_err() {
        return Err(());
    }
    if socket
        .send(Message::Binary(writer.into_inner().unwrap()))
        .await
        .is_err()
    {
        return Err(());
    }
    Ok(())
}

async fn handle_sending_a_message(
    mut author: String,
    mut content: String,
    msg_send: &mpsc::Sender<ChatRoomAction>,
    socket: &mut WebSocket,
) -> Result<(), ()> {
    truncate_author_and_content_strings(&mut author, &mut content);
    author = ammonia::clean_text(&author);
    content = ammonia::clean_text(&content);
    if msg_send
        .send(ChatRoomAction::CreateMessage(Box::new(ChatMessage {
            content,
            author,
        })))
        .await
        .is_err()
    {
        return Err(());
    }
    if socket.send(Message::Binary([].to_vec())).await.is_err() {
        return Err(());
    }
    Ok(())
}

fn truncate_author_and_content_strings(author: &mut String, content: &mut String) {
    if author.len() > 30 {
        *author = truncate(author, 30);
        author.push_str("...")
    }
    if content.len() > 280 {
        *content = truncate(content, 280);
        content.push_str("...")
    }
}

fn truncate(s: &mut String, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_string(),
        Some((idx, _)) => (s[..idx]).to_owned(),
    }
}
