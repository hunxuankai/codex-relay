use codex_relay_release_console_lib::models::{
    ReleaseEvent, ReleaseModelError, ReleasePhase, ReleaseSession,
};

#[test]
fn release_phase_allows_only_declared_forward_transitions() {
    let ordered = [
        ReleasePhase::Idle,
        ReleasePhase::Inspected,
        ReleasePhase::Planned,
        ReleasePhase::ApplyingCandidate,
        ReleasePhase::LocalChecks,
        ReleasePhase::LocalBuild,
        ReleasePhase::SourceAudit,
        ReleasePhase::Committed,
        ReleasePhase::Pushed,
        ReleasePhase::WorkflowQueued,
        ReleasePhase::WorkflowRunning,
        ReleasePhase::AuditingDraft,
        ReleasePhase::AwaitingPublishApproval,
        ReleasePhase::Publishing,
        ReleasePhase::VerifyingPublishedRelease,
        ReleasePhase::MonitoringCleanup,
    ];

    for phases in ordered.windows(2) {
        assert_eq!(phases[0].transition_to(phases[1]), Ok(phases[1]));
    }
    assert_eq!(
        ReleasePhase::MonitoringCleanup.transition_to(ReleasePhase::Completed),
        Ok(ReleasePhase::Completed)
    );
    assert_eq!(
        ReleasePhase::MonitoringCleanup.transition_to(ReleasePhase::CompletedWithWarnings),
        Ok(ReleasePhase::CompletedWithWarnings)
    );
    assert_eq!(
        ReleasePhase::Planned.transition_to(ReleasePhase::LocalBuild),
        Err(ReleaseModelError::InvalidPhaseTransition {
            from: ReleasePhase::Planned,
            to: ReleasePhase::LocalBuild,
        })
    );
    assert_eq!(
        ReleasePhase::Completed.transition_to(ReleasePhase::Failed),
        Err(ReleaseModelError::InvalidPhaseTransition {
            from: ReleasePhase::Completed,
            to: ReleasePhase::Failed,
        })
    );

    for phase in ordered {
        assert_eq!(
            phase.transition_to(ReleasePhase::Failed),
            Ok(ReleasePhase::Failed)
        );
        assert_eq!(
            phase.transition_to(ReleasePhase::Cancelled),
            Ok(ReleasePhase::Cancelled)
        );
    }
}

#[test]
fn invalid_phase_transition_exposes_stable_code() {
    let error = ReleasePhase::Completed
        .transition_to(ReleasePhase::Publishing)
        .unwrap_err();

    assert_eq!(error.code(), "RELEASE_PHASE_TRANSITION_INVALID");
}

#[test]
fn release_phase_uses_camel_case_dto_values() {
    let json = serde_json::to_string(&ReleasePhase::AwaitingPublishApproval).unwrap();
    assert_eq!(json, "\"awaitingPublishApproval\"");
    assert_eq!(
        serde_json::from_str::<ReleasePhase>(&json).unwrap(),
        ReleasePhase::AwaitingPublishApproval
    );
}

#[test]
fn session_update_event_keeps_the_json_contract_without_inlining_the_large_session() {
    let session = ReleaseSession::new("session-size", r"D:\safe-temp\repository", "0.5.0");
    let event = ReleaseEvent::SessionUpdated {
        session: Box::new(session),
    };

    let value = serde_json::to_value(&event).unwrap();

    assert_eq!(value["kind"], "sessionUpdated");
    assert_eq!(value["session"]["id"], "session-size");
    assert!(std::mem::size_of::<ReleaseEvent>() < 256);
}
