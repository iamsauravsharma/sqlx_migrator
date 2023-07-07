var searchIndex = JSON.parse('{\
"sqlx_migrator":{"doc":"Library to create sqlx migration using rust code instead …","t":"AAAAACDNNNDNELLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLFLLLLLLLLLLLLLLLLLLNNNNENNNNNNLLLLLLLLLLLLLLMMMDIKLLLLLLLLLLLLLKKKLLLLLLLNNIIIDDENLKLLLLLLLLLLLLLKLLLLKLLLLKLLLLKLLLLLLLLLLLLLLKLLLLKLKLLLKLLLLLLLLLLLLKLLLLLLLILK","n":["cli","error","migration","migrator","operation","sqlx","Apply","Apply","Drop","List","Revert","Revert","SubCommand","augment_args","augment_args","augment_args_for_update","augment_args_for_update","augment_subcommands","augment_subcommands_for_update","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","command","command","command_for_update","command_for_update","fmt","fmt","fmt","from","from","from","from_arg_matches","from_arg_matches","from_arg_matches","from_arg_matches_mut","from_arg_matches_mut","from_arg_matches_mut","group_id","group_id","handle_subcommand","has_subcommand","into","into","into","run","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches_mut","update_from_arg_matches_mut","update_from_arg_matches_mut","vzip","vzip","vzip","AppNameNotExists","AppNameRequired","AppliedMigrationExists","BothMigrationTypeApplied","Error","FailedToCreateMigrationPlan","IrreversibleOperation","MigrationNameNotExists","PendingMigrationPresent","SqlxError","UnsupportedDatabase","borrow","borrow_mut","fmt","fmt","from","from","into","provide","source","to_string","try_from","try_into","type_id","vzip","app","app","migration","AppliedMigrationSqlRow","Migration","app","applied_time","borrow","borrow_mut","clone","clone_into","eq","eq","from","from_row","hash","id","into","is_atomic","name","operations","parents","replaces","run_before","to_owned","try_from","try_into","type_id","vzip","All","Apply","DatabaseOperation","Info","Migrate","Migrator","Plan","PlanType","Revert","add_migration","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migrations","apply_all","apply_migration","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fmt","fmt","from","from","from","generate_migration_plan","into","into","into","list_applied_migrations","lock","lock","lock","lock","lock","migrations","migrations","migrations_mut","migrations_mut","new","new","pool","pool","revert_all","revert_migration","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","unlock","unlock","unlock","unlock","unlock","vzip","vzip","vzip","Operation","down","up"],"q":[[0,"sqlx_migrator"],[6,"sqlx_migrator::cli"],[67,"sqlx_migrator::error"],[92,"sqlx_migrator::error::Error"],[95,"sqlx_migrator::migration"],[121,"sqlx_migrator::migrator"],[207,"sqlx_migrator::operation"]],"d":["Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait","Migrator module","Operation module","","CLI struct for apply subcommand","Apply migrations","Drop sqlx information migrations table. Needs all …","List migrations along with their status","CLI struct for revert subcommand","Revert migrations","Subcommand for sqlx migrator cli","","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","","","","","","","","","Handle all subcommand operations","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Run full cli by parsing args with help of migrator. If you …","","","","","","","","","","","","","","","","","","","Error when provided app name doesn’t exists","Error when migration name is only present but not app name","Error when applied migrations exists","Error when migration plan has applied replaces migrations …","Error enum to store different types of error","Error for failed to create migrations plan","Error for irreversible operation","Error when provided migration name doesn’t exists for app","Error for pending migration present","Error type created from error raised by sqlx","Error when unsupported database is used as any database","","","","","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","","","","","","","","Name of app","Name of app","Name of migration","Migration struct created from sql table. Struct contains 4 …","Trait for migration","Migration app name. Can be name of folder or library where …","Return migration applied time","","","","","","","Returns the argument unchanged.","","","Return id value present on database","Calls <code>U::from(self)</code>.","Whether migration is atomic or not. By default it is true","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Replace certain migrations. If any one of listed migration …","Run before certain migration. This can be helpful in …","","","","","","Plan type used when listing all migration in chronological …","Plan type used when listing migrations which can be applied","Trait which is implemented for database for performing …","Info trait which implements some of database agnostic …","Migrate trait which migrate a database according to …","Migrator struct which store migrations graph and …","Struct which determine type of plan to use","Type of plan which needs to be generate","Plan type when listing migrations which can be reverted","Add single migration to migrator object","Add migration to migration table","","","","","Add vector of migrations to Migrator object","Apply missing migration","Apply given migration and add it to applied migration table","","","","","","","Delete migration from migration table","","","","","Drop migration table if migration table exists","","","","","Ensure migration table is created before running …","","","","","List all applied migrations from database as struct","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Generate migration plan for according to plan type. …","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","List all applied migrations. Returns a vector of migration","Lock database while doing migrations so no two migrations …","","","","","Return migrations","","Return mutable reference of migrations","","Create new migrator from pool","Create new plan using plan type, app name and migration …","Return pool of database","","Revert all applied migration from database","Revert provided migration and remove migration from table","","","","","","","","","","Unlock locked database","","","","","","","","Trait for operation","Down command to be executed during migration rollback. If …","Up command to be executed during migration apply"],"i":[0,0,0,0,0,0,0,2,2,2,0,2,0,5,6,5,6,2,2,2,5,6,2,5,6,5,6,5,6,2,5,6,2,5,6,2,5,6,2,5,6,5,6,2,2,2,5,6,0,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,14,14,14,14,0,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,44,45,45,0,0,25,22,22,22,22,22,25,22,22,22,25,22,22,25,25,25,25,25,25,22,22,22,22,22,39,39,0,0,0,0,0,0,39,46,47,35,35,35,35,46,12,12,35,39,40,35,39,40,47,35,35,35,35,47,35,35,35,35,47,35,35,35,35,47,35,35,35,35,39,40,35,39,40,12,35,39,40,12,47,35,35,35,35,46,35,46,35,35,40,46,35,12,12,35,39,40,35,39,40,35,39,40,47,35,35,35,35,35,39,40,0,30,30],"f":[0,0,0,0,0,0,0,0,0,0,0,0,0,[1,1],[1,1],[1,1],[1,1],[1,1],[1,1],[[]],[[]],[[]],[[]],[[]],[[]],[[],1],[[],1],[[],1],[[],1],[[2,3],4],[[5,3],4],[[6,3],4],[[]],[[]],[[]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[[],[[11,[10]]]],[[],[[11,[10]]]],[[2,[13,[12]]],[[9,[14]]]],[15,16],[[]],[[]],[[]],[[[13,[12]]],[[9,[14]]]],[[],9],[[],9],[[],9],[[],9],[[],9],[[],9],[[],17],[[],17],[[],17],[[2,7],[[9,[8]]]],[[5,7],[[9,[8]]]],[[6,7],[[9,[8]]]],[[2,7],[[9,[8]]]],[[5,7],[[9,[8]]]],[[6,7],[[9,[8]]]],[[]],[[]],[[]],0,0,0,0,0,0,0,0,0,0,0,[[]],[[]],[[14,3],4],[[14,3],4],[[]],[18,14],[[]],[19],[14,[[11,[20]]]],[[],21],[[],9],[[],9],[[],17],[[]],0,0,0,0,0,[[],15],[22,[[24,[23]]]],[[]],[[]],[22,22],[[]],[[25,25],16],[[22,[13,[25]]],16],[[]],[26,[[27,[22]]]],[[25,28]],[22,29],[[]],[[],16],[[],15],[[],[[31,[[13,[30]]]]]],[[],[[31,[[13,[25]]]]]],[[],[[31,[[13,[25]]]]]],[[],[[31,[[13,[25]]]]]],[[]],[[],9],[[],9],[[],17],[[]],0,0,0,0,0,0,0,0,0,[[[13,[25]]]],[[[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[34]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[36]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[37]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[38]],[13,[25]]],[[33,[[13,[32]]]]]],[[[31,[[13,[25]]]]]],[[],[[33,[[13,[32]]]]]],[[[13,[25]]],[[33,[[13,[32]]]]]],[[]],[[]],[[]],[[]],[[]],[[]],[[[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[36]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[37]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[34]],[13,[25]]],[[33,[[13,[32]]]]]],[[[35,[38]],[13,[25]]],[[33,[[13,[32]]]]]],[[],[[33,[[13,[32]]]]]],[[[35,[38]]],[[33,[[13,[32]]]]]],[[[35,[34]]],[[33,[[13,[32]]]]]],[[[35,[37]]],[[33,[[13,[32]]]]]],[[[35,[36]]],[[33,[[13,[32]]]]]],[[],[[33,[[13,[32]]]]]],[[[35,[34]]],[[33,[[13,[32]]]]]],[[[35,[36]]],[[33,[[13,[32]]]]]],[[[35,[37]]],[[33,[[13,[32]]]]]],[[[35,[38]]],[[33,[[13,[32]]]]]],[[],[[33,[[13,[32]]]]]],[[[35,[38]]],[[33,[[13,[32]]]]]],[[[35,[34]]],[[33,[[13,[32]]]]]],[[[35,[37]]],[[33,[[13,[32]]]]]],[[[35,[36]]],[[33,[[13,[32]]]]]],[[39,3],4],[[40,3],4],[[]],[[]],[[]],[40,[[33,[[13,[32]]]]]],[[]],[[]],[[]],[[],[[33,[[13,[32]]]]]],[[],[[33,[[13,[32]]]]]],[[[35,[36]]],[[33,[[13,[32]]]]]],[[[35,[38]]],[[33,[[13,[32]]]]]],[[[35,[34]]],[[33,[[13,[32]]]]]],[[[35,[37]]],[[33,[[13,[32]]]]]],[[],[[41,[[13,[25]]]]]],[[[35,[42]]],[[41,[[13,[25]]]]]],[[],[[41,[[13,[25]]]]]],[[[35,[42]]],[[41,[[13,[25]]]]]],[[[43,[42]]],[[35,[42]]]],[[39,[11,[21]],[11,[21]]],[[9,[40,14]]]],[[],43],[[[35,[42]]],[[43,[42]]]],[[],[[33,[[13,[32]]]]]],[[[13,[25]]],[[33,[[13,[32]]]]]],[[],9],[[],9],[[],9],[[],9],[[],9],[[],9],[[],17],[[],17],[[],17],[[],[[33,[[13,[32]]]]]],[[[35,[37]]],[[33,[[13,[32]]]]]],[[[35,[36]]],[[33,[[13,[32]]]]]],[[[35,[38]]],[[33,[[13,[32]]]]]],[[[35,[34]]],[[33,[[13,[32]]]]]],[[]],[[]],[[]],0,[[],[[33,[[13,[32]]]]]],[[],[[33,[[13,[32]]]]]]],"c":[],"p":[[3,"Command"],[4,"SubCommand"],[3,"Formatter"],[6,"Result"],[3,"Apply"],[3,"Revert"],[3,"ArgMatches"],[6,"Error"],[4,"Result"],[3,"Id"],[4,"Option"],[8,"Migrate"],[3,"Box"],[4,"Error"],[15,"str"],[15,"bool"],[3,"TypeId"],[4,"Error"],[3,"Demand"],[8,"Error"],[3,"String"],[3,"AppliedMigrationSqlRow"],[3,"Utc"],[3,"DateTime"],[8,"Migration"],[8,"Row"],[6,"Result"],[8,"Hasher"],[15,"i32"],[8,"Operation"],[3,"Vec"],[8,"Future"],[3,"Pin"],[3,"Any"],[3,"Migrator"],[3,"MySql"],[3,"Postgres"],[3,"Sqlite"],[4,"PlanType"],[3,"Plan"],[3,"HashSet"],[8,"Database"],[3,"Pool"],[13,"AppNameNotExists"],[13,"MigrationNameNotExists"],[8,"Info"],[8,"DatabaseOperation"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
