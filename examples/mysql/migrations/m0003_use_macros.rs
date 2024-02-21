pub(crate) struct M0003Operation;

sqlx_migrator::mysql_operation!(
    M0003Operation,
    "INSERT INTO sample (id, name) VALUES (999, 'Another text')",
    "DELETE FROM sample WHERE id = 999"
);

pub(crate) struct M0003Migration;

sqlx_migrator::mysql_migration!(
    "main",
    M0003Migration,
    sqlx_migrator::vec_box![crate::migrations::m0002_with_parents::M0002Migration],
    sqlx_migrator::vec_box![M0003Operation]
);
