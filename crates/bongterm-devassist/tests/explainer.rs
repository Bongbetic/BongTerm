mod ai {
    pub mod explainer {
        use bongterm_devassist::DevassistError;
        use bongterm_devassist::ai::{AiSuggestion, Explainer};
        use bongterm_storage_api::{BlockId, CommandBlockRow, PaneId, SessionId};
        use bongterm_test_kit::mocks::ai_backend::MockAiBackend;
        use uuid::Uuid;

        fn failed_block(exit: i64, cmd: &str) -> CommandBlockRow {
            CommandBlockRow {
                id: BlockId(Uuid::nil()),
                pane_id: PaneId(Uuid::nil()),
                session_id: SessionId(Uuid::nil()),
                command: cmd.to_string(),
                exit_code: Some(exit),
                started_at: time::OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(time::OffsetDateTime::UNIX_EPOCH),
            }
        }

        #[test]
        fn explainer_is_offered_only_for_nonzero_exit() {
            assert!(Explainer::is_explainable(&failed_block(1, "cargo build")));
            assert!(Explainer::is_explainable(&failed_block(127, "frobnicate")));
            assert!(!Explainer::is_explainable(&failed_block(0, "cargo build")));
        }

        #[test]
        fn explainer_builds_context_from_block_and_transcript() {
            let backend = MockAiBackend::available(AiSuggestion {
                command: String::new(),
                explanation: "command not found / not in PATH".to_string(),
            });
            let explainer = Explainer::new(Box::new(backend));
            let block = failed_block(127, "frobnicate --help");
            let result = explainer
                .explain(&block, "frobnicate: command not found")
                .expect("explain should succeed");
            assert!(result.explanation.contains("not found"));
        }

        #[test]
        fn explainer_refuses_zero_exit() {
            let backend = MockAiBackend::available(AiSuggestion {
                command: String::new(),
                explanation: "n/a".to_string(),
            });
            let explainer = Explainer::new(Box::new(backend));
            let block = failed_block(0, "ls");
            assert!(matches!(
                explainer.explain(&block, "ok"),
                Err(DevassistError::Parse(_))
            ));
        }
    }
}
