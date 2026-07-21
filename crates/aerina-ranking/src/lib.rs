use aerina_domain::{EloEntry, ModelPresetId, RankingEvent};
use std::collections::HashMap;

const K: f64 = 32.0;

pub fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((rating_b - rating_a) / 400.0))
}

pub fn rebuild_elo(events: &[RankingEvent]) -> Vec<EloEntry> {
    let mut map: HashMap<(ModelPresetId, String, Option<String>), EloEntry> = HashMap::new();

    for event in events {
        let kind_key = format!("{:?}", event.arena_kind);
        let winner_key = (
            event.winner_preset_id,
            kind_key.clone(),
            event.category.clone(),
        );
        let loser_key = (event.loser_preset_id, kind_key, event.category.clone());

        let winner_rating = map
            .entry(winner_key.clone())
            .or_insert(EloEntry {
                model_preset_id: event.winner_preset_id,
                arena_kind: event.arena_kind,
                category: event.category.clone(),
                rating: 1000.0,
                games: 0,
                wins: 0,
            })
            .rating;

        let loser_rating = map
            .entry(loser_key.clone())
            .or_insert(EloEntry {
                model_preset_id: event.loser_preset_id,
                arena_kind: event.arena_kind,
                category: event.category.clone(),
                rating: 1000.0,
                games: 0,
                wins: 0,
            })
            .rating;

        let exp_w = expected_score(winner_rating, loser_rating);
        let exp_l = expected_score(loser_rating, winner_rating);

        if let Some(winner) = map.get_mut(&winner_key) {
            winner.rating += K * (1.0 - exp_w);
            winner.games += 1;
            winner.wins += 1;
        }
        if let Some(loser) = map.get_mut(&loser_key) {
            loser.rating += K * (0.0 - exp_l);
            loser.games += 1;
        }
    }

    let mut entries = map.into_values().collect::<Vec<_>>();
    entries.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use aerina_domain::*;
    use chrono::Utc;

    #[test]
    fn multi_candidate_elo_updates_winner_and_loser() {
        let winner = ModelPresetId::new();
        let loser = ModelPresetId::new();
        let events = vec![RankingEvent {
            id: RankingEventId::new(),
            workspace_id: WorkspaceId::new(),
            arena_kind: ArenaKind::Text,
            category: Some("general".into()),
            winner_preset_id: winner,
            loser_preset_id: loser,
            round_id: RoundId::new(),
            created_at: Utc::now(),
        }];
        let board = rebuild_elo(&events);
        assert_eq!(board.len(), 2);
        let winner_entry = board.iter().find(|e| e.model_preset_id == winner).unwrap();
        let loser_entry = board.iter().find(|e| e.model_preset_id == loser).unwrap();
        assert!(winner_entry.rating > 1000.0);
        assert!(loser_entry.rating < 1000.0);
        assert_eq!(winner_entry.wins, 1);
        assert!(expected_score(1200.0, 1000.0) > 0.5);
    }
}
