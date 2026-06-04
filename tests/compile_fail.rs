#[test]
fn pg_constraint_typo_suggestions() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipped pg_constraint_typo_suggestions: DATABASE_URL is not set");
        return;
    }

    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/*.rs");
}
