mod jobs {
    pub mod runner {
        use bongterm_devassist::jobs::{
            JobId, JobOutcome, JobRunner, JobSpec, JobState, Notifier, Toast, ToastKind,
        };
        use bongterm_test_kit::mocks::notifier::MockNotifier;

        #[test]
        fn mock_notifier_records_toasts() {
            let n = MockNotifier::new();
            n.notify(&Toast {
                kind: ToastKind::Success,
                title: "BongTerm".to_string(),
                body: "done".to_string(),
            });
            assert_eq!(n.toasts().len(), 1);
            assert_eq!(n.toasts()[0].kind, ToastKind::Success);
        }

        #[test]
        fn runner_emits_success_toast_on_zero_exit() {
            let notifier = MockNotifier::new();
            let runner = JobRunner::new(&notifier);
            let spec = JobSpec {
                id: JobId(uuid::Uuid::nil()),
                label: "echo".to_string(),
                command: "echo".to_string(),
                args: vec![],
                cwd: None,
            };
            let final_state = runner.finish(&spec, JobOutcome::Exited { code: 0 });
            assert_eq!(final_state, JobState::Succeeded);
            assert_eq!(notifier.toasts().len(), 1);
            assert_eq!(notifier.toasts()[0].kind, ToastKind::Success);
        }

        #[test]
        fn runner_emits_failure_toast_on_nonzero_exit() {
            let notifier = MockNotifier::new();
            let runner = JobRunner::new(&notifier);
            let spec = JobSpec {
                id: JobId(uuid::Uuid::nil()),
                label: "sleep 3 && exit 1".to_string(),
                command: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 3 && exit 1".to_string()],
                cwd: None,
            };
            let final_state = runner.finish(&spec, JobOutcome::Exited { code: 1 });
            assert_eq!(final_state, JobState::Failed { exit_code: 1 });
            assert_eq!(notifier.toasts()[0].kind, ToastKind::Failure);
        }

        #[test]
        fn runner_spawn_failure_yields_failure_toast() {
            let notifier = MockNotifier::new();
            let runner = JobRunner::new(&notifier);
            let spec = JobSpec {
                id: JobId(uuid::Uuid::nil()),
                label: "broken".to_string(),
                command: "definitely-not-a-real-binary-xyz".to_string(),
                args: vec![],
                cwd: None,
            };
            let final_state = runner.finish(&spec, JobOutcome::SpawnError("not found".to_string()));
            assert!(matches!(final_state, JobState::Failed { .. }));
            assert_eq!(notifier.toasts()[0].kind, ToastKind::Failure);
        }
    }
}
