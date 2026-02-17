use proc_macro::TokenStream;
use quote::quote;
use syn::{
    LitStr,
    parse_macro_input,
};

#[proc_macro]
pub fn pg_constraint(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);

    #[cfg(feature = "validate")]
    {
        let name = lit.value();
        let validation = match std::env::var("DATABASE_URL") {
            Ok(url) => validate_constraint_exists(&url, &name),
            Err(_) => Ok(()), // no DB: skip validation, same as sqlx offline
        };
        match validation {
            Ok(()) => quote! { #lit }.into(),
            Err(e) => {
                let msg = format!("pg_constraint!: {e}");
                quote! {
                    compile_error!(#msg)
                }
                .into()
            }
        }
    }

    #[cfg(not(feature = "validate"))]
    quote! { #lit }.into()
}

#[cfg(feature = "validate")]
fn validate_constraint_exists(database_url: &str, name: &str) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async {
        let pool: sqlx::PgPool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .map_err(|e| e.to_string())?;

        // Names that can appear in db_err.constraint():
        // 1. pg_constraint.conname - all constraints
        // 2. Unique index names (CREATE UNIQUE INDEX name ...)
        let exists: bool = sqlx::query_scalar(
            "
            SELECT exists(
                SELECT 1 FROM pg_constraint WHERE conname = $1
                UNION ALL
                SELECT 1
                FROM pg_indexes i
                JOIN pg_class c ON c.relname = i.indexname
                    AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
                JOIN pg_index ind ON ind.indexrelid = c.oid
                WHERE ind.indisunique AND i.indexname = $1
            )
            ",
        )
        .bind(name)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

        if exists {
            return Ok(());
        }

        let suggestions = get_similar_constraints(&pool, name).await?;

        let hint = if suggestions.is_empty() {
            format!("no constraint or unique index named \"{name}\"")
        } else {
            format!(
                "no constraint or unique index named \"{name}\". Did you mean: {}?",
                suggestions.join(", ")
            )
        };

        Err(hint)
    })
}

#[cfg(feature = "validate")]
async fn get_similar_constraints(
    pool: &sqlx::PgPool,
    constraint_name: &str,
) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = sqlx::query_scalar(
        "
        SELECT conname AS name FROM pg_constraint
        UNION ALL
        SELECT indexname AS name FROM pg_indexes i
        JOIN pg_class c ON c.relname = i.indexname
          AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
        JOIN pg_index ind ON ind.indexrelid = c.oid
        WHERE ind.indisunique
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    names.sort_by_key(|candidate| strsim::levenshtein(constraint_name, candidate));
    Ok(names.into_iter().take(5).collect())
}
