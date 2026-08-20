//! Contract tests for the invoke/event boundary: backend payloads must
//! deserialize into the shared UI DTOs in `crates/wisp-dto`. The UI cannot
//! link native crates (wasm32), so its DTOs mirror backend shapes by hand;
//! these tests turn silent serde drift into a compile-time/test failure.
//!
//! Pattern: build the backend value → `serde_json::to_value` (what tauri IPC
//! sends) → deserialize into the `superscience_dto` type → assert field fidelity.

use serde_json::json;

fn roundtrip<T: serde::Serialize, D: serde::de::DeserializeOwned>(value: &T) -> D {
    let json = serde_json::to_value(value).expect("backend value must serialize");
    serde_json::from_value(json).expect("UI DTO must accept the backend payload")
}

#[test]
fn ssh_host_contract() {
    let backend = crate::ssh_hosts::SshHost {
        alias: "gpu".into(),
        host_name: Some("10.0.0.5".into()),
        user: Some("alice".into()),
        port: Some(2222),
        identity_file: None,
        notes: Some("lab box".into()),
        auth_method: Some("password".into()),
        has_password: true,
        password: Some("secret".into()),
    };
    let json = serde_json::to_value(&backend).unwrap();
    // Write-only secret must never cross the boundary.
    assert!(json.get("password").is_none());
    // The UI's password placeholder depends on this flag being serialized.
    assert_eq!(json.get("has_password"), Some(&json!(true)));

    let dto: superscience_dto::SshHost = serde_json::from_value(json).unwrap();
    assert_eq!(dto.alias, "gpu");
    assert_eq!(dto.host_name.as_deref(), Some("10.0.0.5"));
    assert_eq!(dto.user.as_deref(), Some("alice"));
    assert_eq!(dto.port, Some(2222));
    assert_eq!(dto.notes.as_deref(), Some("lab box"));
    assert_eq!(dto.auth_method.as_deref(), Some("password"));
    assert!(dto.has_password);
    assert_eq!(dto.password, None);
}

#[test]
fn model_profile_contract() {
    let backend = crate::models::ModelProfile {
        id: "p1".into(),
        label: "Fast".into(),
        provider: "openai".into(),
        api_url: "https://api.example.com".into(),
        endpoint_suffix: "/gateway".into(),
        model: "gpt-test".into(),
        has_api_key: true,
        active: true,
        max_tokens: 4096,
        context_window: 200_000,
        reasoning_effort: "high".into(),
        supports_vision: true,
        use_for_vision: true,
        use_for_image_generation: false,
        image_size: String::new(),
        image_quality: String::new(),
        image_aspect_ratio: String::new(),
        image_resolution: String::new(),
        use_for_video_generation: true,
        video_duration_secs: Some(8),
        video_aspect_ratio: Some("9:16".into()),
        video_resolution: Some("720p".into()),
    };
    let dto: superscience_dto::ModelProfile = roundtrip(&backend);
    assert_eq!(dto.id, "p1");
    assert_eq!(dto.label, "Fast");
    assert_eq!(dto.provider, "openai");
    assert_eq!(dto.api_url, "https://api.example.com");
    assert_eq!(dto.endpoint_suffix, "/gateway");
    assert_eq!(dto.model, "gpt-test");
    assert!(dto.has_api_key);
    assert!(dto.active);
    assert_eq!(dto.max_tokens, 4096);
    assert_eq!(dto.context_window, 200_000);
    assert_eq!(dto.reasoning_effort, "high");
    assert!(dto.supports_vision);
    assert!(dto.use_for_vision);
    assert!(!dto.use_for_image_generation);
    assert!(dto.use_for_video_generation);
    assert_eq!(dto.video_duration_secs, Some(8));
    assert_eq!(dto.video_aspect_ratio.as_deref(), Some("9:16"));
    assert_eq!(dto.video_resolution.as_deref(), Some("720p"));
    assert!(superscience_dto::is_video_generation_model(
        "xai/grok-imagine-video-1.5-preview"
    ));
    assert!(!superscience_dto::is_video_generation_model(
        "grok-imagine-video-2.0"
    ));
    assert_eq!(superscience_dto::VIDEO_ASPECT_RATIOS.len(), 5);
    assert_eq!(superscience_dto::VIDEO_RESOLUTIONS.len(), 3);
    assert_eq!(superscience_dto::VIDEO_DURATION_MIN_SECS, 1);
    assert_eq!(superscience_dto::VIDEO_DURATION_MAX_SECS, 15);
}

#[test]
fn share_social_copy_contract() {
    let backend = crate::share_social::ShareSocialCopy {
        platform: crate::share_social::ShareSocialPlatform::Xiaohongshu,
        highlights: vec![crate::share_social::ShareSocialHighlight {
            title: "Clean peak".into(),
            why: "The 530 nm assignment is unambiguous.".into(),
            message_indexes: vec![1, 3],
        }],
        variants: vec![crate::share_social::ShareSocialVariant {
            title: "Spectrum note".into(),
            body: "主峰在 530 nm。".into(),
            hashtags: vec!["#RNA".into()],
        }],
    };
    let json = serde_json::to_value(&backend).unwrap();
    assert_eq!(json.get("platform"), Some(&json!("xiaohongshu")));
    assert!(json.get("highlights").is_some());
    assert!(json.get("variants").is_some());
    let dto: superscience_dto::ShareSocialCopy = serde_json::from_value(json).unwrap();
    assert_eq!(
        dto.platform,
        superscience_dto::ShareSocialPlatform::Xiaohongshu
    );
    assert_eq!(dto.highlights[0].title, "Clean peak");
    assert_eq!(dto.highlights[0].message_indexes, vec![1, 3]);
    assert_eq!(dto.variants[0].body, "主峰在 530 nm。");
    assert_eq!(dto.variants[0].hashtags, vec!["#RNA".to_string()]);
}

#[test]
fn execution_context_contract() {
    let backend = superscience_store::ExecutionContext {
        id: "ssh:gpu".into(),
        kind: superscience_store::ExecutionContextKind::Ssh,
        label: "GPU box".into(),
        config_json: "{\"alias\":\"gpu\"}".into(),
        capabilities_json: "{}".into(),
        last_probe_at: Some(1_700_000_000),
        last_probe_status: Some("ok".into()),
        last_probe_error: None,
        created_at: 1_700_000_000,
        updated_at: 1_700_000_001,
    };
    let dto: superscience_dto::ExecutionContext = roundtrip(&backend);
    assert_eq!(dto.id, "ssh:gpu");
    // Backend enum serializes lowercase; the UI matches on these strings.
    assert_eq!(dto.kind, "ssh");
    assert_eq!(dto.label, "GPU box");
    assert_eq!(dto.config_json, "{\"alias\":\"gpu\"}");
    assert_eq!(dto.capabilities_json, "{}");
    assert_eq!(dto.last_probe_status.as_deref(), Some("ok"));
    assert_eq!(dto.last_probe_error, None);
}

#[test]
fn run_summary_contract() {
    let backend = superscience_store::RunSummary {
        id: "run-1".into(),
        frame_id: Some("frame-1".into()),
        context_id: "ssh:gpu".into(),
        title: "align reads".into(),
        kind: "shell".into(),
        status: superscience_store::RunStatus::TimedOut,
        created_at: 1,
        started_at: Some(2),
        ended_at: Some(3),
        exit_code: Some(124),
        remote_workdir: Some("/scratch/run-1".into()),
        timeout_secs: Some(60),
        last_polled_at: Some(4),
        last_poll_error: Some("ssh dropped".into()),
        progress_json: "{}".into(),
        harvested_at: None,
        cleaned_at: None,
        cleanup_error: None,
        output_fingerprint: "abc".into(),
    };
    let dto: superscience_dto::RunSummary = roundtrip(&backend);
    assert_eq!(dto.id, "run-1");
    assert_eq!(dto.frame_id.as_deref(), Some("frame-1"));
    assert_eq!(dto.context_id, "ssh:gpu");
    assert_eq!(dto.title, "align reads");
    assert_eq!(dto.kind, "shell");
    // Backend enum serializes snake_case; the UI matches on these strings.
    assert_eq!(dto.status, "timed_out");
    assert_eq!(dto.exit_code, Some(124));
    assert_eq!(dto.remote_workdir.as_deref(), Some("/scratch/run-1"));
    assert_eq!(dto.last_poll_error.as_deref(), Some("ssh dropped"));
    assert_eq!(dto.output_fingerprint, "abc");
}

#[test]
fn project_transfer_progress_contract() {
    let backend = crate::project_transfer::ProjectTransferProgress {
        direction: "export",
        stage: "copying",
        project_id: Some("proj-1".into()),
        completed_files: 3,
        total_files: Some(10),
        completed_bytes: 1024,
        total_bytes: Some(4096),
        current_path: Some("data/reads.fq".into()),
    };
    let json = serde_json::to_value(&backend).unwrap();
    // Event payload is camelCase on the wire.
    assert!(json.get("projectId").is_some());
    assert!(json.get("completedFiles").is_some());

    let dto: superscience_dto::ProjectTransferProgress = serde_json::from_value(json).unwrap();
    assert_eq!(dto.direction, "export");
    assert_eq!(dto.stage, "copying");
    assert_eq!(dto.project_id.as_deref(), Some("proj-1"));
    assert_eq!(dto.completed_files, 3);
    assert_eq!(dto.total_files, Some(10));
    assert_eq!(dto.completed_bytes, 1024);
    assert_eq!(dto.total_bytes, Some(4096));
    assert_eq!(dto.current_path.as_deref(), Some("data/reads.fq"));
    assert!(!dto.is_complete());
    assert!(!dto.is_failed());
}
