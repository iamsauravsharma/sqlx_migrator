var searchIndex = JSON.parse('{\
"sqlx_migrator":{"doc":"Library to create sqlx migration using rust code instead …","t":[0,0,0,0,0,0,0,2,5,4,13,13,11,11,11,11,11,11,11,11,11,11,11,11,11,12,16,8,10,11,11,11,10,11,11,13,16,13,8,4,13,11,10,11,11,11,11,11,10,10,10,11,11,11,11,11,10,10,10,11,11,11,11,11,11,16,8,10,10,0,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,0,3,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11],"n":["cli","error","migration","migrator","operation","postgres","sqlite","sqlx","run","Error","FailedToCreateMigrationPlan","SqlxError","borrow","borrow_mut","fmt","fmt","from","from","into","source","to_string","try_from","try_into","type_id","vzip","0","Database","Migration","app","eq","full_name","hash","name","operations","parents","Apply","Database","Full","Migrator","PlanType","Revert","add_migration","add_migration_to_db_table","add_migrations","apply_all","apply_migration","borrow","borrow_mut","delete_migration_from_db_table","ensure_migration_table_exists","fetch_applied_migration_from_db","fmt","from","generate_migration_plan","into","list_applied_migrations","migrations","migrations_mut","pool","revert_all","revert_migration","try_from","try_into","type_id","vzip","Database","Operation","down","up","migrator","Migrator","add_migration_to_db_table","borrow","borrow_mut","delete_migration_from_db_table","ensure_migration_table_exists","fetch_applied_migration_from_db","from","into","migrations","migrations_mut","new","pool","try_from","try_into","type_id","vzip","migrator","Migrator","add_migration_to_db_table","borrow","borrow_mut","delete_migration_from_db_table","ensure_migration_table_exists","fetch_applied_migration_from_db","from","into","migrations","migrations_mut","new","pool","try_from","try_into","type_id","vzip"],"q":["sqlx_migrator","","","","","","","","sqlx_migrator::cli","sqlx_migrator::error","","","","","","","","","","","","","","","","sqlx_migrator::error::Error","sqlx_migrator::migration","","","","","","","","","sqlx_migrator::migrator","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","sqlx_migrator::operation","","","","sqlx_migrator::postgres","sqlx_migrator::postgres::migrator","","","","","","","","","","","","","","","","","sqlx_migrator::sqlite","sqlx_migrator::sqlite::migrator","","","","","","","","","","","","","","","",""],"d":["Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait","migrator module","Operation module","Postgres module","sqlite module","","Run cli by parsing args with help of migrator","Error enum to store different types of error","Error for failed to create migrations plan from cyclic …","Error type created from error raised by sqlx","","","","","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","","","","","","","","Type of database to be used","Trait for migration","Migration app name. Can be name of folder or library where …","","Full name of migration. Determined from app and name …","","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Apply plan. Plan containing migrations which can be applied","Database type","Full plan. Plan containing all migrations according to …","Migrator trait","Type of plan used to generate migrations","Revert plan. Plan containing migrations which can be …","Add single migration to migrator object","Add migration to migration table","Add vector of migrations to Migrator object","Apply missing migration plan","Apply certain migration to database and add it to applied …","","","Delete migration from table","Ensure migration table is created before running …","List all applied migrations from database in string format …","","Returns the argument unchanged.","Generate migration plan for according to plan type. …","Calls <code>U::from(self)</code>.","List all applied migrations. Returns a vector of migration","Return migrations","Return mutable reference of migrations","Return pool of database","Revert all applied migration from database","Revert migration","","","","","Database type to be used","Trait for operation","Down command to be executed during migration rollback","Up command to be executed during migration apply","Postgres migrator module","Migrator struct which store migrations graph and …","","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","Create new migrator from pool","","","","","","Sqlite migrator module","Migrator struct which store migrations graph and …","","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","Create new migrator from pool","","","","",""],"i":[0,0,0,0,0,0,0,0,0,0,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,25,14,0,14,14,14,14,14,14,14,20,1,20,0,0,20,1,1,1,1,1,20,20,1,1,1,20,20,1,20,1,1,1,1,1,1,20,20,20,20,16,0,16,16,0,0,23,23,23,23,23,23,23,23,23,23,23,23,23,23,23,23,0,0,24,24,24,24,24,24,24,24,24,24,24,24,24,24,24,24],"f":[0,0,0,0,0,0,0,0,[[[2,[1]]],3],0,0,0,[[]],[[]],[[4,5],6],[[4,5],6],[[]],[7,4],[[]],[4,[[9,[8]]]],[[],10],[[],11],[[],11],[[],12],[[]],0,0,0,[[],13],[[14,14],15],[[],10],[14],[[],13],[[],[[17,[[2,[16]]]]]],[[],[[17,[[2,[14]]]]]],0,0,0,0,0,0,[[[2,[14]]]],[[13,18],[[19,[[2,[3]]]]]],[[[17,[[2,[14]]]]]],[[],[[19,[[2,[3]]]]]],[2,[[19,[[2,[3]]]]]],[[]],[[]],[[13,18],[[19,[[2,[3]]]]]],[[],[[19,[[2,[3]]]]]],[[],[[19,[[2,[3]]]]]],[[20,5],6],[[]],[20,[[19,[[2,[3]]]]]],[[]],[[],[[19,[[2,[3]]]]]],[[],21],[[],21],[[],22],[[],[[19,[[2,[3]]]]]],[2,[[19,[[2,[3]]]]]],[[],11],[[],11],[[],12],[[]],0,0,[18,[[19,[[2,[3]]]]]],[18,[[19,[[2,[3]]]]]],0,0,[[23,13,18],[[19,[[2,[3]]]]]],[[]],[[]],[[23,13,18],[[19,[[2,[3]]]]]],[23,[[19,[[2,[3]]]]]],[23,[[19,[[2,[3]]]]]],[[]],[[]],[23,21],[23,21],[22,23],[23,22],[[],11],[[],11],[[],12],[[]],0,0,[[24,13,18],[[19,[[2,[3]]]]]],[[]],[[]],[[24,13,18],[[19,[[2,[3]]]]]],[24,[[19,[[2,[3]]]]]],[24,[[19,[[2,[3]]]]]],[[]],[[]],[24,21],[24,21],[22,24],[24,22],[[],11],[[],11],[[],12],[[]]],"p":[[8,"Migrator"],[3,"Box"],[8,"Future"],[4,"Error"],[3,"Formatter"],[6,"Result"],[4,"Error"],[8,"Error"],[4,"Option"],[3,"String"],[4,"Result"],[3,"TypeId"],[15,"str"],[8,"Migration"],[15,"bool"],[8,"Operation"],[3,"Vec"],[3,"Transaction"],[3,"Pin"],[4,"PlanType"],[3,"HashSet"],[3,"Pool"],[3,"Migrator"],[3,"Migrator"],[13,"SqlxError"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
