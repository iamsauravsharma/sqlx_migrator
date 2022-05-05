use sqlx::Postgres;
use sqlx_migrator::migration::Migration;

mod m0001;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Database = Postgres>>> {
    vec![Box::new(m0001::M0001Migration)]
}
