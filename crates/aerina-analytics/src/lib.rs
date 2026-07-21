use aerina_domain::*;
use chrono::Utc;

pub fn new_event(input: NewAnalyticsEvent) -> AnalyticsEvent {
    AnalyticsEvent {
        id: AnalyticsEventId::new(),
        event_type: input.event_type,
        workspace_id: input.workspace_id,
        conversation_id: input.conversation_id,
        round_id: input.round_id,
        candidate_id: input.candidate_id,
        payload: input.payload,
        created_at: Utc::now(),
    }
}
