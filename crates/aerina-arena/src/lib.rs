use aerina_domain::*;
use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;

pub fn draw_arena_slots(
    request: &ArenaDrawRequest,
    presets: &[ResolvedModelPreset],
) -> Result<Vec<ArenaSlot>> {
    let filtered = filter_by_capabilities(presets, &request.capability_requirements)
        .into_iter()
        .filter(|preset| request.candidate_pool.contains(&preset.preset_id))
        .collect::<Vec<_>>();

    if filtered.len() < request.slot_count {
        return Err(anyhow!(
            "not enough models in pool after capability filter: have {}, need {}",
            filtered.len(),
            request.slot_count
        ));
    }

    let mut rng = rand::rng();
    let mut pool = filtered;
    pool.shuffle(&mut rng);

    if !request.allow_same_provider {
        let mut selected = Vec::new();
        let mut used_providers = std::collections::HashSet::new();
        for preset in pool {
            if used_providers.insert(preset.provider_id) {
                selected.push(preset);
            }
            if selected.len() == request.slot_count {
                break;
            }
        }
        if selected.len() < request.slot_count {
            return Err(anyhow!("not enough distinct providers for arena draw"));
        }
        pool = selected;
    } else {
        pool.truncate(request.slot_count);
    }

    Ok(pool
        .into_iter()
        .enumerate()
        .map(|(index, preset)| ArenaSlot {
            slot_label: format!("{}", (b'A' + index as u8) as char),
            model_preset_id: preset.preset_id,
            provider_id: preset.provider_id,
            model_name: preset.model_name,
        })
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_requires_enough_models() {
        let presets = vec![ResolvedModelPreset {
            preset_id: ModelPresetId::new(),
            provider_id: ProviderId::new(),
            provider_kind: ProviderKind::OpenAiCompatible,
            base_url: "http://localhost".into(),
            api_key: None,
            model_name: "a".into(),
            display_name: "A".into(),
            capabilities: vec![CapabilityTag::Text],
            temperature: None,
            system_prompt: None,
        }];
        let request = ArenaDrawRequest {
            kind: ArenaKind::Text,
            candidate_pool: presets.iter().map(|p| p.preset_id).collect(),
            slot_count: 2,
            capability_requirements: vec![CapabilityTag::Text],
            allow_same_provider: true,
        };
        assert!(draw_arena_slots(&request, &presets).is_err());
    }
}
