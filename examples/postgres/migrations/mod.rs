use sqlx::Postgres;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::vec_box;

pub(crate) mod m0001_simple;
pub(crate) mod m0002_with_parents;
pub(crate) mod m0003_use_macros;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Postgres>>> {
    vec_box![
        m0001_simple::M0001Migration,
        m0002_with_parents::M0002Migration,
        m0003_use_macros::M0003Migration,
    ]
}
