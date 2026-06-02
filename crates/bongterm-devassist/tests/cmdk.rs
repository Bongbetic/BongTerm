mod ai {
    pub mod cmdk {
        use bongterm_devassist::ai::{AiContext, AiSuggestion, CmdKError, CmdKSession, CmdKState};
        use bongterm_test_kit::mocks::ai_backend::MockAiBackend;

        fn ctx() -> AiContext {
            AiContext {
                cwd: "C:\\proj".to_string(),
                shell: "pwsh".to_string(),
                failed_command: None,
                transcript_tail: String::new(),
            }
        }

        #[test]
        fn fresh_session_has_no_runnable_command() {
            let backend = MockAiBackend::available(AiSuggestion {
                command: "Get-ChildItem | Sort-Object Length".to_string(),
                explanation: "lists files by size".to_string(),
            });
            let mut session = CmdKSession::new(Box::new(backend));
            assert!(matches!(
                session.confirm_run(),
                Err(CmdKError::NothingToRun)
            ));
        }

        #[test]
        fn preview_does_not_mark_runnable_until_confirmed() {
            let backend = MockAiBackend::available(AiSuggestion {
                command: "ls -la".to_string(),
                explanation: "list".to_string(),
            });
            let mut session = CmdKSession::new(Box::new(backend));
            let preview = session
                .request_preview("list files", ctx())
                .expect("preview should succeed");
            assert_eq!(preview.command, "ls -la");
            assert_eq!(session.state(), CmdKState::Previewed);
            let cmd = session.confirm_run().expect("confirm should succeed");
            assert_eq!(cmd, "ls -la");
            assert_eq!(session.state(), CmdKState::Confirmed);
        }

        #[test]
        fn unavailable_backend_yields_unavailable_state() {
            let backend = MockAiBackend::unavailable("Claude Code not installed");
            let mut session = CmdKSession::new(Box::new(backend));
            let err = session.request_preview("anything", ctx()).unwrap_err();
            assert!(matches!(err, CmdKError::Unavailable(_)));
            assert_eq!(session.state(), CmdKState::Unavailable);
            assert!(matches!(
                session.confirm_run(),
                Err(CmdKError::NothingToRun)
            ));
        }
    }
}
