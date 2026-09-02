//! Ability-card metadata derived from the capability catalog for usage reporting.

use crate::capabilities_home::{capability_catalog, CapabilityAction, CapabilityTile};
use crate::i18n::{t, Locale};

pub const RESEARCH_DIRECTOR_ID: &str = "research-director";

const DIRECTOR_KICKOFF_PROMPT: &str = "caps.prompt.director_kickoff";

/// Metadata for a reportable ability card (catalog tile or synthetic entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityCardMeta {
    pub id: &'static str,
    pub title_key: &'static str,
    pub blurb_key: &'static str,
}

impl From<&CapabilityTile> for AbilityCardMeta {
    fn from(tile: &CapabilityTile) -> Self {
        Self {
            id: tile.id,
            title_key: tile.title_key,
            blurb_key: tile.blurb_key,
        }
    }
}

fn research_director_meta() -> AbilityCardMeta {
    AbilityCardMeta {
        id: RESEARCH_DIRECTOR_ID,
        title_key: "caps.tile.director.title",
        blurb_key: "home.director_cta_hint",
    }
}

/// All reportable cards: catalog tiles plus the home-page research director CTA.
pub fn ability_card_registry() -> Vec<AbilityCardMeta> {
    let mut cards: Vec<_> = capability_catalog()
        .iter()
        .map(AbilityCardMeta::from)
        .collect();
    cards.push(research_director_meta());
    cards
}

pub fn ability_card_by_id(id: &str) -> Option<AbilityCardMeta> {
    if id == RESEARCH_DIRECTOR_ID {
        return Some(research_director_meta());
    }
    capability_catalog()
        .iter()
        .find(|tile| tile.id == id)
        .map(AbilityCardMeta::from)
}

pub fn ability_card_for_action(action: &CapabilityAction) -> Option<AbilityCardMeta> {
    match action {
        CapabilityAction::ComingSoon | CapabilityAction::None => None,
        CapabilityAction::GuidedChat {
            prompt_key: DIRECTOR_KICKOFF_PROMPT,
            skill: None,
            specialist: None,
        } => Some(research_director_meta()),
        CapabilityAction::GuidedChat {
            prompt_key,
            skill,
            specialist,
        } => capability_catalog()
            .iter()
            .find(|tile| match tile.action {
                CapabilityAction::GuidedChat {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                    specialist: tile_specialist,
                } => {
                    tile_prompt == *prompt_key
                        && tile_skill == *skill
                        && tile_specialist == *specialist
                }
                CapabilityAction::InstallThenGuided {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                } => {
                    tile_prompt == *prompt_key
                        && Some(tile_skill) == *skill
                        && specialist.is_none()
                }
                _ => false,
            })
            .map(AbilityCardMeta::from),
        CapabilityAction::InstallThenGuided { .. }
        | CapabilityAction::NewChat
        | CapabilityAction::OpenRuntimeSetup
        | CapabilityAction::OpenSettings { .. }
        | CapabilityAction::OpenPanel(_)
        | CapabilityAction::OpenDemo => capability_catalog()
            .iter()
            .find(|tile| tile.action == *action)
            .map(AbilityCardMeta::from),
    }
}

pub fn ability_card_for_guided(
    prompt_key: &'static str,
    skill: Option<&'static str>,
    specialist: Option<&'static str>,
) -> Option<AbilityCardMeta> {
    ability_card_for_action(&CapabilityAction::GuidedChat {
        prompt_key,
        skill,
        specialist,
    })
}

/// Chinese title for TCTOKEN usage reporting (matches admin dashboard locale).
pub fn ability_card_api_name(meta: &AbilityCardMeta) -> String {
    t(Locale::Zh, meta.title_key)
}

pub fn ability_card_api_blurb(meta: &AbilityCardMeta) -> String {
    t(Locale::Zh, meta.blurb_key)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_report_ability_card_usage(_meta: &AbilityCardMeta) {}

#[cfg(target_arch = "wasm32")]
pub fn spawn_report_ability_card_usage(meta: &AbilityCardMeta) {
    use crate::bindings::invoke;
    use leptos::spawn_local;
    use serde_wasm_bindgen::to_value;
    use wasm_bindgen::JsValue;

    let card_id = meta.id.to_string();
    let card_name = ability_card_api_name(meta);
    spawn_local(async move {
        let _ = invoke(
            "report_ability_card_usage",
            to_value(&serde_json::json!({ "cardId": card_id, "cardName": card_name })).unwrap(),
        )
        .await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_maybe_report_ability_card_resume(_frame_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub fn spawn_maybe_report_ability_card_resume(frame_id: &str) {
    use crate::bindings::invoke;
    use leptos::spawn_local;
    use serde_wasm_bindgen::to_value;
    use wasm_bindgen::JsValue;

    let frame_id = frame_id.to_string();
    spawn_local(async move {
        let _ = invoke(
            "maybe_report_ability_card_resume",
            to_value(&serde_json::json!({ "frameId": frame_id })).unwrap(),
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_covers_catalog_with_unique_ids() {
        let registry = ability_card_registry();
        let catalog_len = capability_catalog().len();
        assert_eq!(registry.len(), catalog_len + 1);
        let ids: HashSet<_> = registry.iter().map(|meta| meta.id).collect();
        assert_eq!(ids.len(), registry.len());
        for tile in capability_catalog() {
            assert!(ids.contains(tile.id), "missing catalog tile {}", tile.id);
        }
        assert!(ids.contains(RESEARCH_DIRECTOR_ID));
    }

    #[test]
    fn ability_card_for_action_resolves_panel_and_settings_tiles() {
        let remote = ability_card_for_action(&CapabilityAction::OpenSettings {
            section: "environments",
        })
        .expect("remote compute");
        assert_eq!(remote.id, "remote-compute");

        let graph = ability_card_for_action(&CapabilityAction::OpenPanel(
            crate::capabilities_home::CapabilityPanel::Graph,
        ))
        .expect("graph");
        assert_eq!(graph.id, "research-graph");
    }

    #[test]
    fn ability_card_for_action_resolves_guided_skill_tiles() {
        let handwriting = ability_card_for_action(&CapabilityAction::GuidedChat {
            prompt_key: "caps.skill.handwriting_extract.prompt",
            skill: Some("handwriting-extract"),
            specialist: Some("handwriting_extract"),
        })
        .expect("handwriting");
        assert_eq!(handwriting.id, "handwriting-extract");

        let topic = ability_card_for_action(&CapabilityAction::GuidedChat {
            prompt_key: "caps.skill.topic_coach.prompt",
            skill: Some("topic-coach"),
            specialist: None,
        })
        .expect("topic coach");
        assert_eq!(topic.id, "topic-coach");
    }

    #[test]
    fn director_kickoff_maps_to_synthetic_card() {
        let meta = ability_card_for_guided(DIRECTOR_KICKOFF_PROMPT, None, None).unwrap();
        assert_eq!(meta.id, RESEARCH_DIRECTOR_ID);
        assert_eq!(ability_card_api_name(&meta), "科研主任");
    }
}
