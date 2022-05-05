use sqlx_migrator::migrator::Migrator as MigratorTrait;
use sqlx_migrator::postgres::migrator::Migrator;

mod migrations;
#[tokio::main]
async fn main() {
    let uri = std::env::var("POSTGRES_DATABASE_URL").unwrap();
    let mut migrator = Migrator::new_from_uri(&uri).await.unwrap();
    migrator.add_migrations(migrations::migrations());
    migrator.apply_all().await.unwrap();
    migrator.revert_all().await.unwrap();
}
