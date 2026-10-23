# Usage

Define an [`Operation`] (the `up` / `down` work) and group operations into a
[`Migration`]:

```rust,ignore
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

struct CreateSample;

#[async_trait::async_trait]
impl Operation<sqlx::Postgres> for CreateSample {
    async fn up(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY)")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}

struct FirstMigration;

impl Migration<sqlx::Postgres> for FirstMigration {
    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "first_migration"
    }

    // `app` + `name` must be unique. Parents run first; a `(app, name)` tuple
    // can stand in for a parent you cannot import.
    fn parents(&self) -> Vec<Box<dyn Migration<sqlx::Postgres>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<sqlx::Postgres>>> {
        vec![Box::new(CreateSample)]
    }
}
```

## Shortcuts

A `(up, down)` pair of strings is a reversible operation, one string an irreversible
one, and `include_sql!` embeds `.up.sql` / `.down.sql` files. The `migration!` macros
cover the simple case; implement the trait by hand for `replaces`, `run_before` or
`is_atomic`.

```rust,ignore
sqlx_migrator::postgres_migration!(
    FirstMigration,
    "main",
    "first_migration",
    sqlx_migrator::vec_box![],
    sqlx_migrator::vec_box![
        (
            "CREATE TABLE sample (id INTEGER PRIMARY KEY)",
            "DROP TABLE sample"
        ),
        "INSERT INTO sample (id) VALUES (1)",
        sqlx_migrator::include_sql!("sql/0001"),
    ]
);
```

## Running

```rust,ignore
use sqlx_migrator::migrator::{Migrator, Plan};

let mut migrator = Migrator::default();
migrator.add_migration(Box::new(FirstMigration)).unwrap();

let report = migrator.run(&mut *conn, &Plan::apply_all()).await.unwrap();
migrator.run(&mut *conn, &Plan::revert_all()).await.unwrap();
```

[`Plan`](crate::migrator::Plan) can also target an app, a migration or a count.
Progress is logged with `tracing`.
