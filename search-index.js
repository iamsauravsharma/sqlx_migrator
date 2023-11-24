var searchIndex = JSON.parse('{\
"sqlx_migrator":{"doc":"Library to create sqlx migration using rust code instead …","t":"CCCCCEFPPPFPGNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNHNNNNNNNNNNNNNNNNNNPPPPPGPPPPPPPPNNNNNNNNNNNNNNOOOOOFKMNNNNNNNNNNNNNMMMNNNNNNNPPKKKFFGPNMNNNNNNNNNNNNNNMNNNNMNNNNMNNNNMNNNNNNNNNNNNNNMNNNNMNMNNNNNNNNNNNNNMNNNNNNNNKNNM","n":["cli","error","migration","migrator","operation","sqlx","Apply","Apply","Drop","List","Revert","Revert","SubCommand","augment_args","augment_args","augment_args_for_update","augment_args_for_update","augment_subcommands","augment_subcommands_for_update","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","command","command","command_for_update","command_for_update","fmt","fmt","fmt","from","from","from","from_arg_matches","from_arg_matches","from_arg_matches","from_arg_matches_mut","from_arg_matches_mut","from_arg_matches_mut","group_id","group_id","handle_subcommand","has_subcommand","into","into","into","run","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches_mut","update_from_arg_matches_mut","update_from_arg_matches_mut","vzip","vzip","vzip","AppNameNotExists","AppNameRequired","AppliedMigrationExists","BothMigrationTypeApplied","CountGreater","Error","FailedToCreateMigrationPlan","IrreversibleOperation","MigrationNameNotExists","NonAsciiAlphaNumeric","PendingMigrationPresent","SqlxError","StdIoError","UnsupportedDatabase","borrow","borrow_mut","fmt","fmt","from","from","from","into","source","to_string","try_from","try_into","type_id","vzip","actual_len","app","app","count","migration","AppliedMigrationSqlRow","Migration","app","applied_time","borrow","borrow_mut","clone","clone_into","eq","eq","from","from_row","hash","id","into","is_atomic","name","operations","parents","replaces","run_before","to_owned","try_from","try_into","type_id","vzip","All","Apply","DatabaseOperation","Info","Migrate","Migrator","Plan","PlanType","Revert","add_migration","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migrations","apply_all","apply_migration","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","default","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fmt","fmt","from","from","from","generate_migration_plan","into","into","into","list_applied_migrations","lock","lock","lock","lock","lock","migrations","migrations","migrations_mut","migrations_mut","new","revert_all","revert_migration","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","unlock","unlock","unlock","unlock","unlock","vzip","vzip","vzip","with_prefix","Operation","down","is_destructible","up"],"q":[[0,"sqlx_migrator"],[6,"sqlx_migrator::cli"],[67,"sqlx_migrator::error"],[95,"sqlx_migrator::error::Error"],[100,"sqlx_migrator::migration"],[126,"sqlx_migrator::migrator"],[211,"sqlx_migrator::operation"],[215,"clap_builder::builder::command"],[216,"core::fmt"],[217,"core::fmt"],[218,"clap_builder"],[219,"core::result"],[220,"clap_builder::util::id"],[221,"core::option"],[222,"alloc::boxed"],[223,"sqlx_core::pool"],[224,"sqlx_core::database"],[225,"core::any"],[226,"sqlx_core::error"],[227,"std::io::error"],[228,"core::error"],[229,"alloc::string"],[230,"sqlx_core::error"],[231,"core::hash"],[232,"alloc::vec"],[233,"core::future::future"],[234,"core::pin"],[235,"sqlx_core::any::database"],[236,"sqlx_mysql::database"],[237,"sqlx_postgres::database"],[238,"sqlx_sqlite::database"],[239,"std::collections::hash::set"],[240,"core::convert"]],"d":["Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait","Migrator module","Operation module","","CLI struct for apply subcommand","Apply migrations","Drop migration information table. Needs all migrations to …","List migrations along with their status and time applied …","CLI struct for revert subcommand","Revert migrations","Subcommand for sqlx migrator cli","","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","","","","","","","","","Handle all subcommand operations","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Run full cli by parsing args with help of migrator. If you …","","","","","","","","","","","","","","","","","","","Error when provided app name doesn’t exists","Error when migration name is only present but not app name","Error when applied migrations exists","Error when migration plan has applied replaces migrations …","Error when count of migrations is big than total number of …","Error enum to store different types of error","Error for failed to create migrations plan","Error for irreversible operation","Error when provided migration name doesn’t exists for app","Error when passed prefix is not alpha numeric","Error for pending migration present","Error type created from error raised by sqlx","Error type created from error raised by std input output","Error when unsupported database is used as any database","","","","","Returns the argument unchanged.","","","Calls <code>U::from(self)</code>.","","","","","","","Actual length of migration","Name of app","Name of app","Count passed in option","Name of migration","Migration struct created from sql table. Struct contains 4 …","Trait for migration","Migration app name. Can be name of folder or library where …","Return migration applied time","","","","","","","Returns the argument unchanged.","","","Return id value present on database","Calls <code>U::from(self)</code>.","Whether migration is atomic or not. By default it is true","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Replace certain migrations. If any one of listed migration …","Run before certain migration. This can be helpful in …","","","","","","Plan type used when listing all migration in chronological …","Plan type used when listing migrations which can be applied","Trait which is implemented for database for performing …","Info trait which implements some of database agnostic …","Migrate trait which migrate a database according to …","Migrator struct which store migrations graph and …","Struct which determine type of plan to use","Type of plan which needs to be generate","Plan type when listing migrations which can be reverted","Add single migration to migrator object","Add migration to migration table","","","","","Add vector of migrations to Migrator object","Apply all migrations which are not applied till now","Apply given migration and add it to applied migration table","","","","","","","","Delete migration from migration table","","","","","Drop migration table if migration table exists","","","","","Ensure migration table is created before running …","","","","","List all applied migrations from database as struct","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Generate migration plan for according to plan type. …","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","List all applied migrations. Returns a vector of migration","Lock database while doing migrations so no two migrations …","","","","","Return migrations","","Return mutable reference of migrations","","Create new plan using plan type, app name and migration …","Revert all applied migration from database","Revert provided migration and remove migration from table","","","","","","","","","","Unlock locked database","","","","","","","","Use prefix for migrator table name only ascii alpha …","Trait for operation","Down command to be executed during migration rollback. If …","Whether up operation is destructible or not. If operation …","Up command to be executed during migration apply"],"i":[0,0,0,0,0,0,0,2,2,2,0,2,0,5,6,5,6,2,2,2,5,6,2,5,6,5,6,5,6,2,5,6,2,5,6,2,5,6,2,5,6,5,6,2,2,2,5,6,0,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,16,16,16,16,16,0,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,46,47,48,46,48,0,0,25,26,26,26,26,26,25,26,26,26,25,26,26,25,25,25,25,25,25,26,26,26,26,26,42,42,0,0,0,0,0,0,42,33,34,38,38,38,38,33,12,12,38,42,43,38,42,43,38,34,38,38,38,38,34,38,38,38,38,34,38,38,38,38,34,38,38,38,38,42,43,38,42,43,12,38,42,43,12,34,38,38,38,38,33,38,33,38,43,12,12,38,42,43,38,42,43,38,42,43,34,38,38,38,38,38,42,43,38,0,31,31,31],"f":[0,0,0,0,0,0,0,0,0,0,0,0,0,[1,1],[1,1],[1,1],[1,1],[1,1],[1,1],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[],1],[[],1],[[],1],[[],1],[[2,3],4],[[5,3],4],[[6,3],4],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[[],[[11,[10]]]],[[],[[11,[10]]]],[[2,[13,[12]],[14,[-1]]],[[9,[15,16]]],17],[18,19],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[[13,[12]],[14,[-1]]],[[9,[15,16]]],17],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,20,[]],[-1,20,[]],[[2,7],[[9,[15,8]]]],[[5,7],[[9,[15,8]]]],[[6,7],[[9,[15,8]]]],[[2,7],[[9,[15,8]]]],[[5,7],[[9,[15,8]]]],[[6,7],[[9,[15,8]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],0,0,0,0,0,0,0,0,0,0,0,0,0,0,[-1,-2,[],[]],[-1,-2,[],[]],[[16,3],4],[[16,3],4],[-1,-1,[]],[21,16],[22,16],[-1,-2,[],[]],[16,[[11,[23]]]],[-1,24,[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,-2,[],[]],0,0,0,0,0,0,0,[25,18],[26,18],[-1,-2,[],[]],[-1,-2,[],[]],[26,26],[[-1,-2],15,[],[]],[[25,25],19],[[26,[13,[25]]],19],[-1,-1,[]],[-1,[[27,[26]]],28],[[25,-1],15,29],[26,30],[-1,-2,[],[]],[25,19],[25,18],[25,[[32,[[13,[31]]]]]],[25,[[32,[[13,[25]]]]]],[25,[[32,[[13,[25]]]]]],[25,[[32,[[13,[25]]]]]],[-1,-2,[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,-2,[],[]],0,0,0,0,0,0,0,0,0,[[33,[13,[25]]],15],[[34,[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[37]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[39]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[40]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[41]],[13,[25]]],[[36,[[13,[35]]]]]],[[33,[32,[[13,[25]]]]],15],[[12,[14,[-1]]],[[36,[[13,[35]]]]],17],[[12,[13,[25]]],[[36,[[13,[35]]]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[],[[38,[-1]]],17],[[34,[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[41]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[37]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[39]],[13,[25]]],[[36,[[13,[35]]]]]],[[[38,[40]],[13,[25]]],[[36,[[13,[35]]]]]],[34,[[36,[[13,[35]]]]]],[[[38,[41]]],[[36,[[13,[35]]]]]],[[[38,[39]]],[[36,[[13,[35]]]]]],[[[38,[40]]],[[36,[[13,[35]]]]]],[[[38,[37]]],[[36,[[13,[35]]]]]],[34,[[36,[[13,[35]]]]]],[[[38,[41]]],[[36,[[13,[35]]]]]],[[[38,[40]]],[[36,[[13,[35]]]]]],[[[38,[37]]],[[36,[[13,[35]]]]]],[[[38,[39]]],[[36,[[13,[35]]]]]],[34,[[36,[[13,[35]]]]]],[[[38,[40]]],[[36,[[13,[35]]]]]],[[[38,[37]]],[[36,[[13,[35]]]]]],[[[38,[39]]],[[36,[[13,[35]]]]]],[[[38,[41]]],[[36,[[13,[35]]]]]],[[42,3],4],[[43,3],4],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[[12,43],[[36,[[13,[35]]]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[12,[[36,[[13,[35]]]]]],[34,[[36,[[13,[35]]]]]],[[[38,[39]]],[[36,[[13,[35]]]]]],[[[38,[41]]],[[36,[[13,[35]]]]]],[[[38,[37]]],[[36,[[13,[35]]]]]],[[[38,[40]]],[[36,[[13,[35]]]]]],[33,[[44,[[13,[25]]]]]],[[[38,[-1]]],[[44,[[13,[25]]]]],17],[33,[[44,[[13,[25]]]]]],[[[38,[-1]]],[[44,[[13,[25]]]]],17],[[42,[11,[24]],[11,[24]]],[[9,[43,16]]]],[[12,[14,[-1]]],[[36,[[13,[35]]]]],17],[[12,[13,[25]]],[[36,[[13,[35]]]]]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,20,[]],[-1,20,[]],[34,[[36,[[13,[35]]]]]],[[[38,[40]]],[[36,[[13,[35]]]]]],[[[38,[39]]],[[36,[[13,[35]]]]]],[[[38,[41]]],[[36,[[13,[35]]]]]],[[[38,[37]]],[[36,[[13,[35]]]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[[38,[-1]],-2],[[9,[[38,[-1]],16]]],17,[[45,[24]]]],0,[31,[[36,[[13,[35]]]]]],[31,19],[31,[[36,[[13,[35]]]]]]],"c":[],"p":[[5,"Command",215],[6,"SubCommand",6],[5,"Formatter",216],[8,"Result",216],[5,"Apply",6],[5,"Revert",6],[5,"ArgMatches",217],[8,"Error",218],[6,"Result",219],[5,"Id",220],[6,"Option",221],[10,"Migrate",126],[5,"Box",222],[5,"Pool",223],[1,"tuple"],[6,"Error",67],[10,"Database",224],[1,"str"],[1,"bool"],[5,"TypeId",225],[6,"Error",226],[5,"Error",227],[10,"Error",228],[5,"String",229],[10,"Migration",100],[5,"AppliedMigrationSqlRow",100],[8,"Result",226],[10,"Row",230],[10,"Hasher",231],[1,"i32"],[10,"Operation",211],[5,"Vec",232],[10,"Info",126],[10,"DatabaseOperation",126],[10,"Future",233],[5,"Pin",234],[5,"Any",235],[5,"Migrator",126],[5,"MySql",236],[5,"Postgres",237],[5,"Sqlite",238],[6,"PlanType",126],[5,"Plan",126],[5,"HashSet",239],[10,"Into",240],[15,"CountGreater",95],[15,"AppNameNotExists",95],[15,"MigrationNameNotExists",95]],"b":[[83,"impl-Display-for-Error"],[84,"impl-Debug-for-Error"],[86,"impl-From%3CError%3E-for-Error"],[87,"impl-From%3CError%3E-for-Error"],[137,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[138,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[139,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[140,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[152,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[153,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[154,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[155,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[157,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[158,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[159,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[160,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[162,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[163,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[164,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[165,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[167,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[168,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[169,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[170,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[182,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[183,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[184,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[185,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[203,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[204,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[205,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[206,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
