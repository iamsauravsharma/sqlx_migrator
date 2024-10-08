pub(crate) struct M0003Migration;

sqlx_migrator::postgres_migration!(
    M0003Migration,
    "main",
    "m0003_use_macros",
    sqlx_migrator::vec_box![("main", "m0002_with_parents")],
    sqlx_migrator::vec_box![(
        "INSERT INTO sample (id, name) VALUES (999, 'Another text')",
        "DELETE FROM sample WHERE id = 999"
    )]
);
