// Intentionally failing test: used to check that the CI fixer (lear.drufiy.com) picks up and repairs a red build. Delete once the probe is done.
#[test]
fn beta3_ci_probe_fails_on_purpose() {
    assert_eq!(2 + 2, 5, "intentional failure for the beta 3 CI-fix probe");
}
