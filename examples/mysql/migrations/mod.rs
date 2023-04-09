use sqlx_migrator::migration::Migration;
use sqlx_migrator::sqlx::MySql;

pub(crate) mod m0001_simple;
pub(crate) mod m0002_with_parents;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<MySql>>> {
    vec![
        Box::new(m0001_simple::M0001Migration),
        Box::new(m0002_with_parents::M0002Migration),
    ]
}
