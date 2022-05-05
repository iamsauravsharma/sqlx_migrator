use sqlx::Postgres;
use sqlx_migrator::migration::Migration;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Database = Postgres>>> {
    vec![]
}
