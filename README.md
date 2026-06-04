# pg-constraint-macro

Compile-time validation for PostgreSQL constraint names used with [sqlx](https://github.com/launchbadge/sqlx) (`DatabaseError::constraint()`).

When `DATABASE_URL` is set at build time, the macro checks that the name exists in your database (constraints and unique indexes). If it is missing, compilation fails with nearby-name suggestions. Without `DATABASE_URL`, the macro passes the string through unchanged.

## Install

```bash
cargo add pg-constraint-macro
```

## Example

```rust
use pg_constraint_macro::pg_constraint;
use sqlx::Error;

fn handle(err: Error) {
    if let Error::Database(db_err) = &err {
        match db_err.constraint() {
            Some(pg_constraint!("users_email_key")) => {
                // handle unique violation on users.email
            }
            _ => {}
        }
    }
}
```

## License

MIT
