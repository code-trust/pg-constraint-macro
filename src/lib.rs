use std::collections::HashSet;
use std::sync::OnceLock;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    LitStr,
    parse_macro_input,
};

static CONSTRAINT_NAMES: OnceLock<Result<HashSet<String>, String>> = OnceLock::new();

#[proc_macro]
pub fn pg_constraint(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);

    let validation = match std::env::var("DATABASE_URL") {
        Ok(url) => validate_constraint_exists(&url, &lit.value()),
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

fn validate_constraint_exists(database_url: &str, name: &str) -> Result<(), String> {
    let constraint_names = get_constraint_names(database_url)?;

    if constraint_names.contains(name) {
        return Ok(());
    }

    let suggestions = get_similar_constraints(constraint_names, name);

    let hint = if suggestions.is_empty() {
        format!("no constraint or unique index named '{name}'")
    } else {
        format!(
            "no constraint or unique index named '{name}'. Did you mean: {}?",
            suggestions.join(", ")
        )
    };

    Err(hint)
}

fn get_constraint_names(database_url: &str) -> Result<&'static HashSet<String>, String> {
    CONSTRAINT_NAMES
        .get_or_init(|| load_constraint_names(database_url))
        .as_ref()
        .map_err(|error| error.clone())
}

fn load_constraint_names(database_url: &str) -> Result<HashSet<String>, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .map_err(|e| e.to_string())?;

        get_constraint_names_from_db(&pool).await
    })
}

async fn get_constraint_names_from_db(pool: &sqlx::PgPool) -> Result<HashSet<String>, String> {
    // Names that can appear in db_err.constraint():
    // 1. pg_constraint.conname - all constraints
    // 2. Unique index names (CREATE UNIQUE INDEX name ...)
    sqlx::query_scalar(
        "
        SELECT conname AS name FROM pg_constraint
        JOIN pg_namespace ON pg_namespace.oid = connamespace
        WHERE nspname NOT IN ('pg_catalog', 'information_schema')
        UNION ALL
        SELECT indexname AS name FROM pg_indexes i
        JOIN pg_namespace n ON n.nspname = i.schemaname
        JOIN pg_class c ON c.relname = i.indexname AND c.relnamespace = n.oid
        JOIN pg_index ind ON ind.indexrelid = c.oid
        WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
        AND ind.indisunique
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
    .map(|names| names.into_iter().collect())
}

fn get_similar_constraints(
    constraint_names: &HashSet<String>,
    constraint_name: &str,
) -> Vec<String> {
    let mut names: Vec<_> = constraint_names.iter().cloned().collect();
    names.sort_by_key(|candidate| strsim::levenshtein(constraint_name, candidate));
    names.into_iter().take(5).collect()
}
