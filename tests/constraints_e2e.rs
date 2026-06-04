use pg_constraint_macro::pg_constraint;
use sqlx::PgPool;

const ITEMS_TABLE: &str = "e2e_test_items";
const TAGS_TABLE: &str = "e2e_test_tags";

fn constraint_from_db_error(err: sqlx::Error) -> String {
    let sqlx::Error::Database(db_err) = err else {
        panic!("expected sqlx::Error::Database, got {err:?}");
    };
    db_err
        .constraint()
        .expect("expected PostgreSQL error to include a constraint name")
        .to_owned()
}

#[sqlx::test(migrations = false, fixtures(path = "fixtures", scripts("schema")))]
async fn table_unique_constraint_violation_matches_pg_constraint(pool: PgPool) {
    sqlx::query(&format!(
        "INSERT INTO {ITEMS_TABLE} (name) VALUES ('alpha'), ('beta')"
    ))
    .execute(&pool)
    .await
    .expect("failed to seed items");

    let err = sqlx::query(&format!(
        "INSERT INTO {ITEMS_TABLE} (name) VALUES ('alpha')"
    ))
    .execute(&pool)
    .await
    .expect_err("duplicate name should violate unique constraint");

    assert_eq!(
        constraint_from_db_error(err),
        pg_constraint!("e2e_test_items_name_key")
    );
}

#[sqlx::test(migrations = false, fixtures(path = "fixtures", scripts("schema")))]
async fn unique_index_violation_matches_pg_constraint(pool: PgPool) {
    sqlx::query(&format!(
        "INSERT INTO {TAGS_TABLE} (label) VALUES ('news'), ('sports')"
    ))
    .execute(&pool)
    .await
    .expect("failed to seed tags");

    let err = sqlx::query(&format!("INSERT INTO {TAGS_TABLE} (label) VALUES ('news')"))
        .execute(&pool)
        .await
        .expect_err("duplicate label should violate unique index");

    assert_eq!(
        constraint_from_db_error(err),
        pg_constraint!("e2e_test_tags_label_idx")
    );
}

#[sqlx::test(migrations = false, fixtures(path = "fixtures", scripts("schema")))]
async fn unrelated_constraint_name_does_not_match_pg_constraint(pool: PgPool) {
    sqlx::query(&format!(
        "INSERT INTO {ITEMS_TABLE} (name) VALUES ('gamma')"
    ))
    .execute(&pool)
    .await
    .expect("failed to seed item");

    let err = sqlx::query(&format!(
        "INSERT INTO {ITEMS_TABLE} (name) VALUES ('gamma')"
    ))
    .execute(&pool)
    .await
    .expect_err("duplicate name should violate unique constraint");

    let constraint = constraint_from_db_error(err);
    assert_ne!(constraint, pg_constraint!("e2e_test_tags_label_idx"));
    assert_eq!(constraint, pg_constraint!("e2e_test_items_name_key"));
}
