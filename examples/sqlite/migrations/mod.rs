use sqlx::Sqlite;
use sqlx_migrator::migration::Migration;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Database = Sqlite>>> {
    vec![]
}
