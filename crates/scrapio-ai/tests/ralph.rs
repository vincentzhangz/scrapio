//! Tests for Ralph extraction module

use scrapio_ai::ralph::{
    ExtractionStatus, RalphInput, RalphInputError, RalphProgress, RalphTarget,
};

#[test]
fn test_ralph_input_parse_prompt_objective() {
    let input = RalphInput::parse("[]", "Extract product name and price").unwrap();
    match input {
        RalphInput::PromptObjective { objective } => {
            assert_eq!(objective, "Extract product name and price");
        }
        _ => panic!("Expected PromptObjective"),
    }
}

#[test]
fn test_ralph_input_parse_target_list() {
    let schema = r#"[{"id": "title", "description": "Extract title"}, {"id": "price", "description": "Extract price"}]"#;
    let input = RalphInput::parse(schema, "").unwrap();
    match input {
        RalphInput::TargetList { targets } => {
            assert_eq!(targets.len(), 2);
            assert_eq!(targets[0].id, "title");
            assert_eq!(targets[1].id, "price");
        }
        _ => panic!("Expected TargetList"),
    }
}

#[test]
fn test_ralph_input_parse_json_schema() {
    let schema = r#"{"type": "object", "required": ["name"], "additionalProperties": false}"#;
    let input = RalphInput::parse(schema, "").unwrap();
    match input {
        RalphInput::JsonExtractionSchema { .. } => {}
        _ => panic!("Expected JsonExtractionSchema"),
    }
}

#[test]
fn test_ralph_input_parse_json_schema_with_properties() {
    let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
    let input = RalphInput::parse(schema, "").unwrap();
    match input {
        RalphInput::TargetList { targets } => {
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].id, "name");
        }
        _ => panic!("Expected TargetList"),
    }
}

#[test]
fn test_ralph_input_parse_explicit_format() {
    let schema = r#"{"type": "prompt_objective", "objective": "Do something"}"#;
    let input = RalphInput::parse(schema, "").unwrap();
    match input {
        RalphInput::PromptObjective { objective } => {
            assert_eq!(objective, "Do something");
        }
        _ => panic!("Expected PromptObjective"),
    }
}

#[test]
fn test_ralph_input_parse_empty_schema_error() {
    let result = RalphInput::parse("", "");
    assert!(result.is_err());
    match result.unwrap_err() {
        RalphInputError::EmptySchema => {}
        e => panic!("Expected EmptySchema, got {:?}", e),
    }
}

#[test]
fn test_ralph_input_parse_invalid_json_error() {
    let result = RalphInput::parse("{invalid json}", "");
    assert!(result.is_err());
}

#[test]
fn test_ralph_input_to_targets_prompt_objective() {
    let input = RalphInput::PromptObjective {
        objective: "Test objective".to_string(),
    };
    let targets = input.to_targets();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "objective");
    assert_eq!(targets[0].description, "Test objective");
}

#[test]
fn test_ralph_input_to_targets_target_list() {
    let input = RalphInput::TargetList {
        targets: vec![
            RalphTarget::new("id1", "desc1"),
            RalphTarget::new("id2", "desc2"),
        ],
    };
    let targets = input.to_targets();
    assert_eq!(targets.len(), 2);
}

#[test]
fn test_ralph_target_validate_non_empty_string() {
    let data = serde_json::json!("some value");
    let mut t = RalphTarget::new("test", "desc");
    t.data = Some(data);
    assert!(t.validate());
}

#[test]
fn test_ralph_target_validate_empty_string() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!(""));
    assert!(!target.validate());
}

#[test]
fn test_ralph_target_validate_non_empty_array() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!(["item1", "item2"]));
    assert!(target.validate());
}

#[test]
fn test_ralph_target_validate_empty_array() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!([]));
    assert!(!target.validate());
}

#[test]
fn test_ralph_target_validate_null() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!(null));
    assert!(!target.validate());
}

#[test]
fn test_ralph_target_validate_with_rule_non_empty() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!("value"));
    target.validation_rule = Some("non_empty".to_string());
    assert!(target.validate());
}

#[test]
fn test_ralph_target_validate_with_rule_required() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!("something"));
    target.validation_rule = Some("required".to_string());
    assert!(target.validate());
}

#[test]
fn test_ralph_target_validate_with_rule_not_empty_string() {
    let mut target = RalphTarget::new("test", "desc");
    target.data = Some(serde_json::json!("  hello  "));
    target.validation_rule = Some("not_empty_string".to_string());
    assert!(target.validate());
}

#[test]
fn test_ralph_target_mark_verified() {
    let mut target = RalphTarget::new("test", "desc");
    target.mark_verified(serde_json::json!({"key": "value"}));
    assert!(target.extracted);
    assert_eq!(target.status, ExtractionStatus::VerifiedSuccess);
    assert!(target.error.is_none());
}

#[test]
fn test_ralph_target_mark_partial() {
    let mut target = RalphTarget::new("test", "desc");
    target.mark_partial(serde_json::json!({"key": "value"}));
    assert!(target.extracted);
    assert_eq!(target.status, ExtractionStatus::PartialSuccess);
}

#[test]
fn test_ralph_progress_all_extracted() {
    let mut progress = RalphProgress::default();
    progress.targets.push(RalphTarget::new("t1", "d1"));
    progress.targets.push(RalphTarget::new("t2", "d2"));

    assert!(!progress.all_extracted());

    progress.mark_extracted("t1", serde_json::json!("value"));
    assert!(!progress.all_extracted());

    progress.mark_extracted("t2", serde_json::json!("value"));
    assert!(progress.all_extracted());
}

#[test]
fn test_ralph_progress_all_verified() {
    let mut progress = RalphProgress::default();
    progress.targets.push(RalphTarget::new("t1", "d1"));
    progress.targets.push(RalphTarget::new("t2", "d2"));

    assert!(!progress.all_verified());

    progress.mark_extracted("t1", serde_json::json!("value1"));
    progress.mark_extracted("t2", serde_json::json!("value2"));

    assert!(progress.all_verified());
}

#[test]
fn test_ralph_progress_all_failed() {
    let mut progress = RalphProgress::default();
    progress.targets.push(RalphTarget::new("t1", "d1"));
    progress.targets.push(RalphTarget::new("t2", "d2"));

    assert!(!progress.all_failed());

    progress.mark_failed("t1", "error 1");
    progress.mark_failed("t2", "error 2");

    assert!(progress.all_failed());
}

#[test]
fn test_ralph_progress_mark_failed() {
    let mut progress = RalphProgress::default();
    progress.targets.push(RalphTarget::new("test", "desc"));

    progress.mark_failed("test", "Something went wrong");

    let target = &progress.targets[0];
    assert_eq!(target.error, Some("Something went wrong".to_string()));
    assert_eq!(target.status, ExtractionStatus::Failed);
}

#[test]
fn test_ralph_progress_from_schema() {
    let schema = r#"[{"id": "a", "description": "A"}, {"id": "b", "description": "B"}]"#;
    let progress = RalphProgress::from_schema(schema, "").unwrap();

    assert_eq!(progress.targets.len(), 2);
    assert_eq!(progress.targets[0].id, "a");
    assert_eq!(progress.targets[1].id, "b");
}

#[test]
fn test_ralph_progress_from_schema_with_prompt() {
    let progress = RalphProgress::from_schema("[]", "My objective").unwrap();

    assert_eq!(progress.targets.len(), 1);
    assert_eq!(progress.targets[0].id, "objective");
    assert_eq!(progress.targets[0].description, "My objective");
}

#[test]
fn test_ralph_progress_next_pending_target() {
    let mut progress = RalphProgress::default();
    progress.targets.push(RalphTarget::new("t1", "d1"));
    progress.targets.push(RalphTarget::new("t2", "d2"));

    let next = progress.next_pending_target();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, "t1");

    progress.mark_extracted("t1", serde_json::json!("value"));

    let next = progress.next_pending_target();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, "t2");
}

#[test]
fn test_extraction_status_default() {
    let status = ExtractionStatus::default();
    assert_eq!(status, ExtractionStatus::Pending);
}

#[test]
fn test_extraction_status_values() {
    use serde_json;

    let pending = serde_json::to_string(&ExtractionStatus::Pending).unwrap();
    assert!(pending.contains("pending"));

    let partial = serde_json::to_string(&ExtractionStatus::PartialSuccess).unwrap();
    assert!(partial.contains("partial_success"));

    let verified = serde_json::to_string(&ExtractionStatus::VerifiedSuccess).unwrap();
    assert!(verified.contains("verified_success"));

    let failed = serde_json::to_string(&ExtractionStatus::Failed).unwrap();
    assert!(failed.contains("failed"));
}
