use sqlx::Sqlite;
use sqlx_migrator::migration::Migration;

pub(crate) mod m0001;
pub(crate) mod m0002;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Database = Sqlite>>> {
    vec![
        Box::new(m0001::M0001Migration),
        Box::new(m0002::M0002Migration),
    ]
}
