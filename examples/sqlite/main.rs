use sqlx_migrator::migrator::Migrator as MigratorTrait;
use sqlx_migrator::sqlite::migrator::Migrator;

mod migrations;
#[tokio::main]
async fn main() {
    let uri = std::env::var("SQLITE_DATABASE_URL").unwrap();
    let pool = sqlx::Pool::connect(&uri).await.unwrap();
    let mut migrator = Migrator::new_from_pool(&pool);
    migrator.add_migrations(migrations::migrations());
    // There are two way to run migration. Either you can create cli as shown below
    sqlx_migrator::cli::run(Box::new(migrator)).await.unwrap();
    // Or you can run migrator apply function as required as commented out part
    // below
    // migrator.apply_all().await.unwrap();
}
