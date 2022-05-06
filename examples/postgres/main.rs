use sqlx_migrator::migrator::Migrator as MigratorTrait;
use sqlx_migrator::postgres::migrator::Migrator;

mod migrations;
#[tokio::main]
async fn main() {
    let uri = std::env::var("SQLITE_DATABASE_URL").unwrap();
    let pool = sqlx::Pool::connect(&uri).await.unwrap();
    let mut migrator = Migrator::new_from_pool(&pool);
    migrator.add_migrations(migrations::migrations());
    migrator.apply_all().await.unwrap();
    sqlx::query("SELECT * FROM sample")
        .execute(&pool)
        .await
        .unwrap();
    migrator.revert_all().await.unwrap();
}
