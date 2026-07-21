use std::path::PathBuf;
use std::sync::Arc;

use aerina_app::{
    ActivityDay, AppState, BackupInfo, ConversationDetail, ConversationExport,
    CreateConversationRequest, CreateProfileRequest, SendMessageRequest, SessionInfo, StatsSummary,
    UpsertMcpServerRequest, UpsertModelPresetRequest, UpsertProviderRequest,
};
use aerina_domain::{
    ArenaKind, Conversation, ConversationMode, ConversationSettings, EloEntry, GenerationEvent,
    McpServer, ModelInfo, ModelPreset, Profile, Provider, VoteKind,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

struct SharedState(pub Arc<AppState>);

#[derive(Debug, Serialize)]
struct CommandError {
    message: String,
}

impl From<anyhow::Error> for CommandError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

#[tauri::command]
async fn session_info(state: State<'_, SharedState>) -> Result<SessionInfo, CommandError> {
    Ok(state.0.session_info().await?)
}

#[tauri::command]
async fn create_profile(
    state: State<'_, SharedState>,
    request: CreateProfileRequest,
) -> Result<SessionInfo, CommandError> {
    Ok(state.0.create_profile(request).await?)
}

#[tauri::command]
async fn rename_profile(
    state: State<'_, SharedState>,
    profile_id: String,
    display_name: String,
) -> Result<Profile, CommandError> {
    Ok(state.0.rename_profile(&profile_id, &display_name).await?)
}

#[tauri::command]
async fn set_profile_avatar(
    state: State<'_, SharedState>,
    profile_id: String,
    data_url: String,
) -> Result<Profile, CommandError> {
    Ok(state.0.set_profile_avatar(&profile_id, &data_url).await?)
}

#[tauri::command]
async fn clear_profile_avatar(
    state: State<'_, SharedState>,
    profile_id: String,
) -> Result<Profile, CommandError> {
    Ok(state.0.clear_profile_avatar(&profile_id).await?)
}

#[tauri::command]
async fn get_profile_avatar_data_url(
    state: State<'_, SharedState>,
    profile_id: String,
) -> Result<Option<String>, CommandError> {
    Ok(state.0.get_profile_avatar_data_url(&profile_id).await?)
}

#[tauri::command]
async fn switch_profile(
    state: State<'_, SharedState>,
    profile_id: String,
) -> Result<SessionInfo, CommandError> {
    Ok(state.0.switch_profile(&profile_id).await?)
}

#[tauri::command]
async fn rename_conversation(
    state: State<'_, SharedState>,
    conversation_id: String,
    title: String,
) -> Result<Conversation, CommandError> {
    Ok(state
        .0
        .rename_conversation(&conversation_id, &title)
        .await?)
}

#[tauri::command]
async fn list_conversations(
    state: State<'_, SharedState>,
) -> Result<Vec<Conversation>, CommandError> {
    Ok(state.0.list_conversations().await?)
}

#[tauri::command]
async fn get_conversation(
    state: State<'_, SharedState>,
    conversation_id: String,
    limit: Option<u32>,
    before_message_id: Option<String>,
) -> Result<ConversationDetail, CommandError> {
    Ok(state
        .0
        .get_conversation_page(&conversation_id, limit, before_message_id.as_deref())
        .await?)
}

#[tauri::command]
async fn create_conversation(
    state: State<'_, SharedState>,
    request: CreateConversationRequest,
) -> Result<Conversation, CommandError> {
    Ok(state.0.create_conversation(request).await?)
}

#[tauri::command]
async fn delete_conversation(
    state: State<'_, SharedState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    state.0.delete_conversation(&conversation_id).await?;
    Ok(())
}

#[tauri::command]
async fn delete_provider(
    state: State<'_, SharedState>,
    provider_id: String,
) -> Result<(), CommandError> {
    state.0.delete_provider(&provider_id).await?;
    Ok(())
}

#[tauri::command]
async fn delete_model_preset(
    state: State<'_, SharedState>,
    preset_id: String,
) -> Result<(), CommandError> {
    state.0.delete_model_preset(&preset_id).await?;
    Ok(())
}

#[tauri::command]
async fn list_mcp_servers(state: State<'_, SharedState>) -> Result<Vec<McpServer>, CommandError> {
    Ok(state.0.list_mcp_servers().await?)
}

#[tauri::command]
async fn upsert_mcp_server(
    state: State<'_, SharedState>,
    request: UpsertMcpServerRequest,
) -> Result<McpServer, CommandError> {
    Ok(state.0.upsert_mcp_server(request).await?)
}

#[tauri::command]
async fn delete_mcp_server(
    state: State<'_, SharedState>,
    server_id: String,
) -> Result<(), CommandError> {
    state.0.delete_mcp_server(&server_id).await?;
    Ok(())
}

#[tauri::command]
async fn list_mcp_tools(
    state: State<'_, SharedState>,
) -> Result<Vec<aerina_domain::McpToolInfo>, CommandError> {
    Ok(state.0.list_mcp_tools().await?)
}

#[tauri::command]
async fn test_mcp_server(
    state: State<'_, SharedState>,
    server_id: String,
) -> Result<String, CommandError> {
    Ok(state.0.test_mcp_server(&server_id).await?)
}

#[tauri::command]
async fn call_mcp_tool(
    state: State<'_, SharedState>,
    request: aerina_domain::McpToolCallRequest,
) -> Result<aerina_domain::McpToolCallResult, CommandError> {
    Ok(state.0.call_mcp_tool(request).await?)
}

#[tauri::command]
async fn list_providers(state: State<'_, SharedState>) -> Result<Vec<Provider>, CommandError> {
    Ok(state.0.list_providers().await?)
}

#[tauri::command]
async fn upsert_provider(
    state: State<'_, SharedState>,
    request: UpsertProviderRequest,
) -> Result<Provider, CommandError> {
    Ok(state.0.upsert_provider(request).await?)
}

#[tauri::command]
async fn list_model_presets(
    state: State<'_, SharedState>,
) -> Result<Vec<ModelPreset>, CommandError> {
    Ok(state.0.list_model_presets().await?)
}

#[tauri::command]
async fn list_provider_models(
    state: State<'_, SharedState>,
    provider_id: String,
) -> Result<Vec<ModelInfo>, CommandError> {
    Ok(state.0.list_provider_models(&provider_id).await?)
}

#[tauri::command]
async fn infer_model_compat(
    state: State<'_, SharedState>,
    model_name: String,
) -> Result<ModelInfo, CommandError> {
    Ok(state.0.infer_model_compat(&model_name))
}

#[tauri::command]
async fn upsert_model_preset(
    state: State<'_, SharedState>,
    request: UpsertModelPresetRequest,
) -> Result<ModelPreset, CommandError> {
    Ok(state.0.upsert_model_preset(request).await?)
}

#[tauri::command]
async fn update_conversation_model(
    state: State<'_, SharedState>,
    conversation_id: String,
    model_preset_id: String,
) -> Result<ConversationSettings, CommandError> {
    Ok(state
        .0
        .update_conversation_model(&conversation_id, &model_preset_id)
        .await?)
}

#[tauri::command]
async fn update_conversation_settings(
    state: State<'_, SharedState>,
    conversation_id: String,
    system_prompt: Option<String>,
    temperature: Option<f32>,
) -> Result<ConversationSettings, CommandError> {
    Ok(state
        .0
        .update_system_prompt(&conversation_id, system_prompt, temperature)
        .await?)
}

#[tauri::command]
async fn cancel_generation(
    state: State<'_, SharedState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    state.0.cancel_generation(&conversation_id).await?;
    Ok(())
}

#[tauri::command]
async fn leaderboard(state: State<'_, SharedState>) -> Result<Vec<EloEntry>, CommandError> {
    Ok(state.0.leaderboard().await?)
}

#[tauri::command]
async fn stats_summary(state: State<'_, SharedState>) -> Result<StatsSummary, CommandError> {
    Ok(state.0.stats_summary().await?)
}

#[tauri::command]
async fn activity_heatmap(
    state: State<'_, SharedState>,
    days: Option<u32>,
) -> Result<Vec<ActivityDay>, CommandError> {
    Ok(state.0.activity_heatmap(days.unwrap_or(365)).await?)
}

#[tauri::command]
async fn set_conversation_models(
    state: State<'_, SharedState>,
    conversation_id: String,
    model_preset_ids: Vec<String>,
    mode: ConversationMode,
) -> Result<ConversationSettings, CommandError> {
    Ok(state
        .0
        .set_conversation_models(&conversation_id, model_preset_ids, mode)
        .await?)
}

#[tauri::command]
async fn commit_candidate(
    state: State<'_, SharedState>,
    conversation_id: String,
    candidate_id: String,
) -> Result<ConversationDetail, CommandError> {
    Ok(state
        .0
        .commit_candidate(&conversation_id, &candidate_id)
        .await?)
}

#[tauri::command]
async fn fork_candidate(
    state: State<'_, SharedState>,
    conversation_id: String,
    candidate_id: String,
) -> Result<ConversationDetail, CommandError> {
    Ok(state
        .0
        .fork_candidate(&conversation_id, &candidate_id)
        .await?)
}

#[tauri::command]
async fn switch_branch(
    state: State<'_, SharedState>,
    conversation_id: String,
    branch_id: String,
) -> Result<ConversationDetail, CommandError> {
    Ok(state.0.switch_branch(&conversation_id, &branch_id).await?)
}

#[tauri::command]
async fn cast_arena_vote(
    state: State<'_, SharedState>,
    conversation_id: String,
    round_id: String,
    vote_kind: VoteKind,
    selected_candidate_id: Option<String>,
) -> Result<ConversationDetail, CommandError> {
    Ok(state
        .0
        .cast_arena_vote(
            &conversation_id,
            &round_id,
            vote_kind,
            selected_candidate_id,
        )
        .await?)
}

#[tauri::command]
async fn configure_arena(
    state: State<'_, SharedState>,
    conversation_id: String,
    pool: Vec<String>,
    slot_count: u32,
    arena_kind: ArenaKind,
    category: Option<String>,
) -> Result<ConversationSettings, CommandError> {
    Ok(state
        .0
        .configure_arena(&conversation_id, pool, slot_count, arena_kind, category)
        .await?)
}

#[tauri::command]
async fn list_round_candidates(
    state: State<'_, SharedState>,
    round_id: String,
) -> Result<Vec<serde_json::Value>, CommandError> {
    Ok(state.0.list_round_candidates(&round_id).await?)
}

#[tauri::command]
async fn export_conversation(
    state: State<'_, SharedState>,
    conversation_id: String,
) -> Result<ConversationExport, CommandError> {
    Ok(state.0.export_conversation(&conversation_id).await?)
}

#[tauri::command]
async fn import_conversation(
    state: State<'_, SharedState>,
    payload: ConversationExport,
) -> Result<Conversation, CommandError> {
    Ok(state.0.import_conversation(payload).await?)
}

#[tauri::command]
async fn get_media_data_url(
    state: State<'_, SharedState>,
    media_id: String,
) -> Result<String, CommandError> {
    Ok(state.0.get_media_data_url(&media_id).await?)
}

#[tauri::command]
async fn regenerate(
    app: AppHandle,
    state: State<'_, SharedState>,
    conversation_id: String,
) -> Result<ConversationDetail, CommandError> {
    let cid = conversation_id.clone();
    let detail = state
        .0
        .regenerate(&conversation_id, move |event: GenerationEvent| {
            let _ = app.emit(
                "generation-event",
                serde_json::json!({
                    "conversationId": cid,
                    "event": event,
                }),
            );
        })
        .await?;
    Ok(detail)
}

#[tauri::command]
async fn edit_user_message(
    app: AppHandle,
    state: State<'_, SharedState>,
    conversation_id: String,
    message_id: String,
    content: String,
) -> Result<ConversationDetail, CommandError> {
    let cid = conversation_id.clone();
    let detail = state
        .0
        .edit_user_message(
            &conversation_id,
            &message_id,
            content,
            move |event: GenerationEvent| {
                let _ = app.emit(
                    "generation-event",
                    serde_json::json!({
                        "conversationId": cid,
                        "event": event,
                    }),
                );
            },
        )
        .await?;
    Ok(detail)
}

#[tauri::command]
async fn retry_candidate(
    app: AppHandle,
    state: State<'_, SharedState>,
    conversation_id: String,
    candidate_id: String,
) -> Result<ConversationDetail, CommandError> {
    let cid = conversation_id.clone();
    let detail = state
        .0
        .retry_candidate(
            &conversation_id,
            &candidate_id,
            move |event: GenerationEvent| {
                let _ = app.emit(
                    "generation-event",
                    serde_json::json!({
                        "conversationId": cid,
                        "event": event,
                    }),
                );
            },
        )
        .await?;
    Ok(detail)
}

#[tauri::command]
async fn create_backup(state: State<'_, SharedState>) -> Result<BackupInfo, CommandError> {
    Ok(state.0.create_backup().await?)
}

#[tauri::command]
async fn list_backups(state: State<'_, SharedState>) -> Result<Vec<BackupInfo>, CommandError> {
    Ok(state.0.list_backups().await?)
}

#[tauri::command]
async fn restore_backup(
    state: State<'_, SharedState>,
    name: String,
) -> Result<String, CommandError> {
    Ok(state.0.restore_backup(&name).await?)
}
#[tauri::command]
async fn send_message(
    app: AppHandle,
    state: State<'_, SharedState>,
    request: SendMessageRequest,
) -> Result<ConversationDetail, CommandError> {
    let conversation_id = request.conversation_id.clone();
    let detail = state
        .0
        .send_message(request, move |event: GenerationEvent| {
            let _ = app.emit(
                "generation-event",
                serde_json::json!({
                    "conversationId": conversation_id,
                    "event": event,
                }),
            );
        })
        .await?;
    Ok(detail)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from(".aerina-data"));
            let state = tauri::async_runtime::block_on(AppState::bootstrap(data_dir))
                .expect("failed to bootstrap aerina app state");
            app.manage(SharedState(Arc::new(state)));

            // Setup tray icon and context menu
            let quit_i =
                tauri::menu::MenuItem::with_id(app, "quit", "退出 (Quit)", true, None::<&str>)?;
            let show_i = tauri::menu::MenuItem::with_id(
                app,
                "show",
                "显示窗口 (Show Window)",
                true,
                None::<&str>,
            )?;
            let menu = tauri::menu::Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.webview_windows().values().next() {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.webview_windows().values().next() {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            session_info,
            create_profile,
            rename_profile,
            set_profile_avatar,
            clear_profile_avatar,
            get_profile_avatar_data_url,
            switch_profile,
            rename_conversation,
            list_conversations,
            get_conversation,
            create_conversation,
            delete_conversation,
            list_providers,
            upsert_provider,
            delete_provider,
            list_model_presets,
            list_provider_models,
            infer_model_compat,
            upsert_model_preset,
            delete_model_preset,
            list_mcp_servers,
            upsert_mcp_server,
            delete_mcp_server,
            list_mcp_tools,
            test_mcp_server,
            call_mcp_tool,
            update_conversation_model,
            update_conversation_settings,
            cancel_generation,
            leaderboard,
            stats_summary,
            activity_heatmap,
            set_conversation_models,
            commit_candidate,
            fork_candidate,
            switch_branch,
            cast_arena_vote,
            configure_arena,
            list_round_candidates,
            export_conversation,
            import_conversation,
            get_media_data_url,
            regenerate,
            edit_user_message,
            retry_candidate,
            create_backup,
            list_backups,
            restore_backup,
            send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
