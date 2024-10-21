use sqlx::Sqlite;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::vec_box;

pub(crate) mod m0001_simple;
pub(crate) mod m0002_with_parents;
pub(crate) mod m0003_use_macros;
pub(crate) mod m0004_complex_operation;
pub(crate) mod m0005_reference_complex;

pub(crate) fn migrations() -> Vec<Box<dyn Migration<Sqlite>>> {
    vec_box![
        m0001_simple::M0001Migration,
        m0002_with_parents::M0002Migration,
        m0003_use_macros::M0003Migration,
        m0004_complex_operation::M0004Migration {
            id: 23,
            message: "Custom String".to_string()
        },
        m0005_reference_complex::M0005Migration,
    ]
}
