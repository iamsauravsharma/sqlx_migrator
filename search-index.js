var searchIndex = JSON.parse('{\
"sqlx_migrator":{"doc":"Library to create sqlx migration using rust code instead …","t":"AAAAACDNNNDNELLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLFLLLLLLLLLLLLLLLLLLNNNNNENNNNNNNNLLLLLLLLLLLLLLMMMMMDIKLLLLLLLLLLLLLKKKLLLLLLLNNIIIDDENLKLLLLLLLLLLLLLLKLLLLKLLLLKLLLLKLLLLLLLLLLLLLLKLLLLKLKLLLLLLLLLLLLLKLLLLLLLLILLK","n":["cli","error","migration","migrator","operation","sqlx","Apply","Apply","Drop","List","Revert","Revert","SubCommand","augment_args","augment_args","augment_args_for_update","augment_args_for_update","augment_subcommands","augment_subcommands_for_update","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","command","command","command_for_update","command_for_update","fmt","fmt","fmt","from","from","from","from_arg_matches","from_arg_matches","from_arg_matches","from_arg_matches_mut","from_arg_matches_mut","from_arg_matches_mut","group_id","group_id","handle_subcommand","has_subcommand","into","into","into","run","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches","update_from_arg_matches_mut","update_from_arg_matches_mut","update_from_arg_matches_mut","vzip","vzip","vzip","AppNameNotExists","AppNameRequired","AppliedMigrationExists","BothMigrationTypeApplied","CountGreater","Error","FailedToCreateMigrationPlan","IrreversibleOperation","MigrationNameNotExists","NonAsciiAlphaNumeric","PendingMigrationPresent","SqlxError","StdIoError","UnsupportedDatabase","borrow","borrow_mut","fmt","fmt","from","from","from","into","source","to_string","try_from","try_into","type_id","vzip","actual_len","app","app","count","migration","AppliedMigrationSqlRow","Migration","app","applied_time","borrow","borrow_mut","clone","clone_into","eq","eq","from","from_row","hash","id","into","is_atomic","name","operations","parents","replaces","run_before","to_owned","try_from","try_into","type_id","vzip","All","Apply","DatabaseOperation","Info","Migrate","Migrator","Plan","PlanType","Revert","add_migration","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migrations","apply_all","apply_migration","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","default","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fmt","fmt","from","from","from","generate_migration_plan","into","into","into","list_applied_migrations","lock","lock","lock","lock","lock","migrations","migrations","migrations_mut","migrations_mut","new","revert_all","revert_migration","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","unlock","unlock","unlock","unlock","unlock","vzip","vzip","vzip","with_prefix","Operation","down","is_destructible","up"],"q":[[0,"sqlx_migrator"],[6,"sqlx_migrator::cli"],[67,"sqlx_migrator::error"],[95,"sqlx_migrator::error::Error"],[100,"sqlx_migrator::migration"],[126,"sqlx_migrator::migrator"],[211,"sqlx_migrator::operation"],[215,"clap_builder::builder::command"],[216,"core::fmt"],[217,"core::fmt"],[218,"clap_builder"],[219,"core::result"],[220,"clap_builder::util::id"],[221,"core::option"],[222,"alloc::boxed"],[223,"sqlx_core::pool"],[224,"sqlx_core::database"],[225,"core::any"],[226,"sqlx_core::error"],[227,"std::io::error"],[228,"core::error"],[229,"alloc::string"],[230,"sqlx_core::error"],[231,"core::hash"],[232,"alloc::vec"],[233,"core::future::future"],[234,"core::pin"],[235,"sqlx_postgres::database"],[236,"sqlx_mysql::database"],[237,"sqlx_core::any::database"],[238,"sqlx_sqlite::database"],[239,"std::collections::hash::set"],[240,"core::convert"]],"d":["Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait","Migrator module","Operation module","","CLI struct for apply subcommand","Apply migrations","Drop migration information table. Needs all migrations to …","List migrations along with their status and time applied …","CLI struct for revert subcommand","Revert migrations","Subcommand for sqlx migrator cli","","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","","","","","","","","","Handle all subcommand operations","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Run full cli by parsing args with help of migrator. If you …","","","","","","","","","","","","","","","","","","","Error when provided app name doesn’t exists","Error when migration name is only present but not app name","Error when applied migrations exists","Error when migration plan has applied replaces migrations …","Error when count of migrations is big than total number of …","Error enum to store different types of error","Error for failed to create migrations plan","Error for irreversible operation","Error when provided migration name doesn’t exists for app","Error when passed prefix is not alpha numeric","Error for pending migration present","Error type created from error raised by sqlx","Error type created from error raised by std input output","Error when unsupported database is used as any database","","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","Actual length of migration","Name of app","Name of app","Count passed in option","Name of migration","Migration struct created from sql table. Struct contains 4 …","Trait for migration","Migration app name. Can be name of folder or library where …","Return migration applied time","","","","","","","Returns the argument unchanged.","","","Return id value present on database","Calls <code>U::from(self)</code>.","Whether migration is atomic or not. By default it is true","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Replace certain migrations. If any one of listed migration …","Run before certain migration. This can be helpful in …","","","","","","Plan type used when listing all migration in chronological …","Plan type used when listing migrations which can be applied","Trait which is implemented for database for performing …","Info trait which implements some of database agnostic …","Migrate trait which migrate a database according to …","Migrator struct which store migrations graph and …","Struct which determine type of plan to use","Type of plan which needs to be generate","Plan type when listing migrations which can be reverted","Add single migration to migrator object","Add migration to migration table","","","","","Add vector of migrations to Migrator object","Apply all migrations which are not applied till now","Apply given migration and add it to applied migration table","","","","","","","","Delete migration from migration table","","","","","Drop migration table if migration table exists","","","","","Ensure migration table is created before running …","","","","","List all applied migrations from database as struct","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Generate migration plan for according to plan type. …","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","List all applied migrations. Returns a vector of migration","Lock database while doing migrations so no two migrations …","","","","","Return migrations","","Return mutable reference of migrations","","Create new plan using plan type, app name and migration …","Revert all applied migration from database","Revert provided migration and remove migration from table","","","","","","","","","","Unlock locked database","","","","","","","","Use prefix for migrator table name only ascii alpha …","Trait for operation","Down command to be executed during migration rollback. If …","Whether up operation is destructible or not. If operation …","Up command to be executed during migration apply"],"i":[0,0,0,0,0,0,0,2,2,2,0,2,0,5,6,5,6,2,2,2,5,6,2,5,6,5,6,5,6,2,5,6,2,5,6,2,5,6,2,5,6,5,6,2,2,2,5,6,0,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,2,5,6,16,16,16,16,16,0,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,16,44,45,46,44,46,0,0,26,25,25,25,25,25,26,25,25,25,26,25,25,26,26,26,26,26,26,25,25,25,25,25,40,40,0,0,0,0,0,0,40,47,48,36,36,36,36,47,12,12,36,40,41,36,40,41,36,48,36,36,36,36,48,36,36,36,36,48,36,36,36,36,48,36,36,36,36,40,41,36,40,41,12,36,40,41,12,48,36,36,36,36,47,36,47,36,41,12,12,36,40,41,36,40,41,36,40,41,48,36,36,36,36,36,40,41,36,0,31,31,31],"f":[0,0,0,0,0,0,0,0,0,0,0,0,0,[1,1],[1,1],[1,1],[1,1],[1,1],[1,1],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[],1],[[],1],[[],1],[[],1],[[2,3],4],[[5,3],4],[[6,3],4],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[7,[[9,[2,8]]]],[7,[[9,[5,8]]]],[7,[[9,[6,8]]]],[[],[[11,[10]]]],[[],[[11,[10]]]],[[2,[13,[12]],[14,[-1]]],[[9,[15,16]]],17],[18,19],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[[13,[12]],[14,[-1]]],[[9,[15,16]]],17],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,20,[]],[-1,20,[]],[[2,7],[[9,[15,8]]]],[[5,7],[[9,[15,8]]]],[[6,7],[[9,[15,8]]]],[[2,7],[[9,[15,8]]]],[[5,7],[[9,[15,8]]]],[[6,7],[[9,[15,8]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],0,0,0,0,0,0,0,0,0,0,0,0,0,0,[-1,-2,[],[]],[-1,-2,[],[]],[[16,3],4],[[16,3],4],[21,16],[22,16],[-1,-1,[]],[-1,-2,[],[]],[16,[[11,[23]]]],[-1,24,[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,-2,[],[]],0,0,0,0,0,0,0,[-1,18,[]],[25,18],[-1,-2,[],[]],[-1,-2,[],[]],[25,25],[[-1,-2],15,[],[]],[[26,26],19],[[25,[13,[26]]],19],[-1,-1,[]],[-1,[[27,[25]]],28],[[26,-1],15,29],[25,30],[-1,-2,[],[]],[-1,19,[]],[-1,18,[]],[-1,[[32,[[13,[31]]]]],[]],[-1,[[32,[[13,[26]]]]],[]],[-1,[[32,[[13,[26]]]]],[]],[-1,[[32,[[13,[26]]]]],[]],[-1,-2,[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,-2,[],[]],0,0,0,0,0,0,0,0,0,[[-1,[13,[26]]],15,[]],[[-1,[13,[26]]],[[34,[[13,[33]]]]],[]],[[[36,[35]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[37]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[38]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[39]],[13,[26]]],[[34,[[13,[33]]]]]],[[-1,[32,[[13,[26]]]]],15,[]],[[-1,[14,[-2]]],[[34,[[13,[33]]]]],[],[]],[[-1,[13,[26]]],[[34,[[13,[33]]]]],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[],[[36,[-1]]],17],[[-1,[13,[26]]],[[34,[[13,[33]]]]],[]],[[[36,[39]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[35]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[37]],[13,[26]]],[[34,[[13,[33]]]]]],[[[36,[38]],[13,[26]]],[[34,[[13,[33]]]]]],[-1,[[34,[[13,[33]]]]],[]],[[[36,[35]]],[[34,[[13,[33]]]]]],[[[36,[39]]],[[34,[[13,[33]]]]]],[[[36,[38]]],[[34,[[13,[33]]]]]],[[[36,[37]]],[[34,[[13,[33]]]]]],[-1,[[34,[[13,[33]]]]],[]],[[[36,[35]]],[[34,[[13,[33]]]]]],[[[36,[39]]],[[34,[[13,[33]]]]]],[[[36,[38]]],[[34,[[13,[33]]]]]],[[[36,[37]]],[[34,[[13,[33]]]]]],[-1,[[34,[[13,[33]]]]],[]],[[[36,[39]]],[[34,[[13,[33]]]]]],[[[36,[37]]],[[34,[[13,[33]]]]]],[[[36,[38]]],[[34,[[13,[33]]]]]],[[[36,[35]]],[[34,[[13,[33]]]]]],[[40,3],4],[[41,3],4],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[[-1,41],[[34,[[13,[33]]]]],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,[[34,[[13,[33]]]]],[]],[-1,[[34,[[13,[33]]]]],[]],[[[36,[39]]],[[34,[[13,[33]]]]]],[[[36,[37]]],[[34,[[13,[33]]]]]],[[[36,[38]]],[[34,[[13,[33]]]]]],[[[36,[35]]],[[34,[[13,[33]]]]]],[-1,[[42,[[13,[26]]]]],[]],[[[36,[-1]]],[[42,[[13,[26]]]]],17],[-1,[[42,[[13,[26]]]]],[]],[[[36,[-1]]],[[42,[[13,[26]]]]],17],[[40,[11,[24]],[11,[24]]],[[9,[41,16]]]],[[-1,[14,[-2]]],[[34,[[13,[33]]]]],[],[]],[[-1,[13,[26]]],[[34,[[13,[33]]]]],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,[[9,[-2]]],[],[]],[-1,20,[]],[-1,20,[]],[-1,20,[]],[-1,[[34,[[13,[33]]]]],[]],[[[36,[38]]],[[34,[[13,[33]]]]]],[[[36,[35]]],[[34,[[13,[33]]]]]],[[[36,[37]]],[[34,[[13,[33]]]]]],[[[36,[39]]],[[34,[[13,[33]]]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[[36,[-1]],-2],[[9,[[36,[-1]],16]]],17,[[43,[24]]]],0,[-1,[[34,[[13,[33]]]]],[]],[-1,19,[]],[-1,[[34,[[13,[33]]]]],[]]],"c":[],"p":[[3,"Command",215],[4,"SubCommand",6],[3,"Formatter",216],[6,"Result",216],[3,"Apply",6],[3,"Revert",6],[3,"ArgMatches",217],[6,"Error",218],[4,"Result",219],[3,"Id",220],[4,"Option",221],[8,"Migrate",126],[3,"Box",222],[3,"Pool",223],[15,"tuple"],[4,"Error",67],[8,"Database",224],[15,"str"],[15,"bool"],[3,"TypeId",225],[4,"Error",226],[3,"Error",227],[8,"Error",228],[3,"String",229],[3,"AppliedMigrationSqlRow",100],[8,"Migration",100],[6,"Result",226],[8,"Row",230],[8,"Hasher",231],[15,"i32"],[8,"Operation",211],[3,"Vec",232],[8,"Future",233],[3,"Pin",234],[3,"Postgres",235],[3,"Migrator",126],[3,"MySql",236],[3,"Any",237],[3,"Sqlite",238],[4,"PlanType",126],[3,"Plan",126],[3,"HashSet",239],[8,"Into",240],[13,"CountGreater",95],[13,"AppNameNotExists",95],[13,"MigrationNameNotExists",95],[8,"Info",126],[8,"DatabaseOperation",126]],"b":[[83,"impl-Display-for-Error"],[84,"impl-Debug-for-Error"],[85,"impl-From%3CError%3E-for-Error"],[86,"impl-From%3CError%3E-for-Error"],[137,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[138,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[139,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[140,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[152,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[153,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[154,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[155,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[157,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[158,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[159,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[160,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[162,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[163,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[164,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[165,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[167,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[168,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[169,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[170,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[182,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[183,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[184,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[185,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[203,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[204,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[205,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[206,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
