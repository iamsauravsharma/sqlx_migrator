searchState.loadedDescShard("sqlx_migrator", 0, "Library to create sqlx migration using rust code instead …\nMacro for implementing the <code>migration</code> macro for the <code>Any</code>.\nModule for creating and running cli with help of migrator\nModule for library error\nModule for defining the <code>Migration</code> trait, which represents …\nMacro for implementing the Migration trait for the …\nMigrator module\nMacro for implementing the <code>migration</code> macro for the <code>MySql</code>.\nModule for defining the <code>Operation</code> trait\nMacro for implementing the <code>migration</code> macro for the <code>Postgres</code>…\nMacro for implementing the <code>migration</code> macro for the <code>Sqlite</code>.\nMacro for vector of <code>Box</code>\nMigration command for performing rust based sqlx migrations\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nParse <code>MigrationCommand</code> and run migration command line …\nRun migration command line interface\nError when applied migrations exists\nError type created from error raised by box error\nError enum to store different types of error\nError for irreversible operation\nError when passed prefix is not alpha numeric\nError for pending migration present\nError generated during planning state\nError type created from error raised by sqlx\nError type created from error raised by std input output\nError when unsupported database is used as any database\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nMessage for error\nStruct representing a migration row from the database.\nTrait for defining database migration\nReturns the application name associated with the migration.\nReturn migration applied time\nReturns the argument unchanged.\nReturn id value present on database\nCalls <code>U::from(self)</code>.\nIndicates whether the migration is atomic. By default, …\nIndicates whether the migration is virtual. By default, …\nReturns the migration name, typically the file name …\nReturns the operations associated with this migration.\nReturns the list of parent migrations.\nReturns the list of migrations that this migration …\nReturns the list of migrations that this migration must …\nThe <code>DatabaseOperation</code> trait defines a set of methods for …\nThe <code>Info</code> trait provides database-agnostic methods for …\nThe <code>Migrate</code> trait defines methods to manage and apply …\nMigrator struct which store migrations graph and …\nStruct that determines the type of migration plan to …\nAdds a single migration to the migrator.\nAdds a migration record to the migration table in the …\nAdds a list of migrations to the migrator.\nCreates a new plan to apply all migrations.\nCreates a new plan to apply a limited number of migrations.\nCreates a new plan to apply a specific migration by name. …\nRemoves a migration record from the migration table in the …\nDrop migration table if migration table exists\nEnsure migration table is created before running …\nSets the plan as a “fake” plan.\nFetches the list of applied migrations from the migration …\nReturns the argument unchanged.\nReturns the argument unchanged.\nGenerate migration plan according to plan.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nLock database while doing migrations so no two migrations …\nReturns a reference to the list of migrations.\nReturns a mutable reference to the list of migrations.\nCreate new migrator\nCreates a new plan to revert all migrations.\nCreates a new plan to revert a limited number of …\nCreates a new plan to revert a specific migration by name. …\nRun provided plan migrations\nGet name of table which is used for storing migrations …\nUnlock locked database\nUse prefix for migrator table name only ascii alpha …\nTrait for defining a database operation.\nThe down method reverses the operation when rolling back …\nIndicates whether the <code>up</code> operation is destructible.\nThe up method executes the operation when applying the …")