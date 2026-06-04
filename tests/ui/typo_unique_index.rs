use pg_constraint_macro::pg_constraint;

fn main() {
    let _ = pg_constraint!("e2e_test_tags_label_id");
}
