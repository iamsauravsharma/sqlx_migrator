#![expect(unused_crate_dependencies)]
//! Example crate for postgres
use sqlx::Postgres;
use sqlx_migrator::cli::MigrationCommand;
use sqlx_migrator::migrator::{Info, Migrator};

mod migrations;
#[tokio::main]
async fn main() {
    let uri = std::env::var("POSTGRES_DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<Postgres>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::default().with_prefix("prefix").unwrap();
    migrator.add_migrations(migrations::migrations()).unwrap();
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
