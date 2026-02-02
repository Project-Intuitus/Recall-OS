use crate::database::{Conversation, Message};
use crate::error::RecallError;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_conversations(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Conversation>, RecallError> {
    state.database.get_all_conversations()
}

#[tauri::command]
pub async fn get_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<Conversation>, RecallError> {
    state.database.get_conversation(&id)
}

#[tauri::command]
pub async fn get_conversation_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> Result<Vec<Message>, RecallError> {
    state.database.get_conversation_messages(&conversation_id)
}

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, Arc<AppState>>,
    title: Option<String>,
) -> Result<Conversation, RecallError> {
    state.database.create_conversation(title.as_deref())
}

#[tauri::command]
pub async fn delete_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), RecallError> {
    state.database.delete_conversation(&id)
}

#[tauri::command]
pub async fn rename_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
    title: String,
) -> Result<(), RecallError> {
    state.database.update_conversation_title(&id, &title)
}
