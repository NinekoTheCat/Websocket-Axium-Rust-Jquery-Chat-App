use serde::Serialize;
use tokio::sync::{broadcast, mpsc, oneshot};
#[derive(Debug)]
pub struct ChatRoom {
    messages: Vec<Message>,
    broadcast_channel: broadcast::Sender<Message>,
    recieve_channel: mpsc::Receiver<ChatRoomAction>,
}
#[derive(Debug)]
pub enum ChatRoomAction {
    CreateMessage(Box<Message>),
    GetMessages {
        recieve_channel: oneshot::Sender<Vec<Message>>,
    },
}
impl ChatRoom {
    pub fn new(
        broadcast_channel: broadcast::Sender<Message>,
        recieve_channel: mpsc::Receiver<ChatRoomAction>,
    ) -> Self {
        Self {
            messages: Vec::new(),
            broadcast_channel,
            recieve_channel,
        }
    }

    pub async fn listen(&mut self) {
        while let Some(action) = self.recieve_channel.recv().await {
            match action {
                ChatRoomAction::CreateMessage(message) => {
                    self.messages.push(*message.clone());
                    self.broadcast_channel.send(*message).unwrap();
                    if self.messages.len() >= 11 {
                        self.messages.remove(0);
                    }
                }
                ChatRoomAction::GetMessages { recieve_channel } => {
                    recieve_channel
                        .send(self.messages.iter().rev().take(10).rev().cloned().collect())
                        .unwrap();
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Message {
    pub(crate) author: String,
    pub(crate) content: String,
}
