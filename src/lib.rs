use std::collections::HashSet;
use std::sync::OnceLock;

use proc_macro::TokenStream;
use quote::quote;
use sqlx::Connection as _;
use syn::{
    LitStr,
    parse_macro_input,
};

static CONSTRAINT_NAMES: OnceLock<Result<HashSet<String>, String>> = OnceLock::new();

#[proc_macro]
pub fn pg_constraint(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let name = lit.value();

    match validate_or_skip(&name) {
        Ok(()) => quote!(#lit).into(),
        Err(e) => {
            let msg = format!("pg_constraint!: {e}");
            quote! {
                compile_error!(#msg)
            }
            .into()
        }
    }
}

fn validate_or_skip(name: &str) -> Result<(), String> {
    if let Some(names) = loaded_constraint_names()? {
        return validate_constraint_name(names, name);
    }

    let Ok(url) = std::env::var("DATABASE_URL") else {
        return Ok(()); // no DB: skip validation
    };

    let names = get_constraint_names(&url)?;
    validate_constraint_name(names, name)
}

fn validate_constraint_name(constraint_names: &HashSet<String>, name: &str) -> Result<(), String> {
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

fn loaded_constraint_names() -> Result<Option<&'static HashSet<String>>, String> {
    match CONSTRAINT_NAMES.get() {
        None => Ok(None),
        Some(Ok(names)) => Ok(Some(names)),
        Some(Err(e)) => Err(e.clone()),
    }
}

fn get_constraint_names(database_url: &str) -> Result<&'static HashSet<String>, String> {
    CONSTRAINT_NAMES
        .get_or_init(|| load_constraint_names(database_url))
        .as_ref()
        .map_err(|error| error.clone())
}

fn load_constraint_names(database_url: &str) -> Result<HashSet<String>, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(async {
            let mut conn = sqlx::PgConnection::connect(database_url)
                .await
                .map_err(|e| e.to_string())?;

            get_constraint_names_from_db(&mut conn).await
        })
}

async fn get_constraint_names_from_db(
    conn: &mut sqlx::PgConnection,
) -> Result<HashSet<String>, String> {
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
    .fetch_all(conn)
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
