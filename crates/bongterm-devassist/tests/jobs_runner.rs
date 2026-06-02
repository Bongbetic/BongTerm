mod jobs {
    pub mod runner {
        use bongterm_devassist::jobs::{Notifier, Toast, ToastKind};
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
    }
}
