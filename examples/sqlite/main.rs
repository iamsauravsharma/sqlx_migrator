#![expect(unused_crate_dependencies)]
//! Example crate for sqlite
use sqlx::Sqlite;
use sqlx_migrator::cli::MigrationCommand;
use sqlx_migrator::migrator::{Info, Migrator};

mod migrations;
#[tokio::main]
async fn main() {
    let uri = std::env::var("SQLITE_DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<Sqlite>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::default();
    migrator.add_migrations(migrations::migrations());
    // There are two way to run migration. Either you can create cli as shown below
    let mut conn = pool.acquire().await.unwrap();
    MigrationCommand::parse_and_run(&mut *conn, Box::new(migrator))
        .await
        .unwrap();
    // Or you can directly use migrator run function instead of creating
    // cli
    // migrator
    //     .run(&mut *conn, sqlx_migrator::migrator::Plan::apply_all())
    //     .await
    //     .unwrap();
    conn.close().await.unwrap();
}
