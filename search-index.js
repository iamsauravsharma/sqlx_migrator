var searchIndex = new Map(JSON.parse('[\
["sqlx_migrator",{"doc":"Library to create sqlx migration using rust code instead …","t":"QQCCCQCQQCQQQQQEQFNNNNNNNNNNNNNNNNNNNNPPPPPGPPPPPPPPNNNNNNNNNNNNNNOOOOOFKMNNNNNNNNNNNNNMMMNNNNNNNPPKKKFFGPNMNNNNNNNNNNNNNNMNNNNMNNNNMNNNNMNNNNNNNNNNNNNNMNNNNMNMNNNNNNNNNNNNNNMNNNNNNNNKNNM","n":["any_migration","any_operation","cli","error","migration","migration","migrator","mysql_migration","mysql_operation","operation","operation","postgres_migration","postgres_operation","sqlite_migration","sqlite_operation","sqlx","vec_box","MigrationCommand","augment_args","augment_args_for_update","borrow","borrow_mut","command","command_for_update","fmt","from","from_arg_matches","from_arg_matches_mut","group_id","into","parse_and_run","run","try_from","try_into","type_id","update_from_arg_matches","update_from_arg_matches_mut","vzip","AppNameNotExists","AppNameRequired","AppliedMigrationExists","BothMigrationTypeApplied","CountGreater","Error","FailedToCreateMigrationPlan","IrreversibleOperation","MigrationNameNotExists","NonAsciiAlphaNumeric","PendingMigrationPresent","SqlxError","StdIoError","UnsupportedDatabase","borrow","borrow_mut","fmt","fmt","from","from","from","into","source","to_string","try_from","try_into","type_id","vzip","actual_len","app","app","count","migration","AppliedMigrationSqlRow","Migration","app","applied_time","borrow","borrow_mut","clone","clone_into","eq","eq","from","from_row","hash","id","into","is_atomic","name","operations","parents","replaces","run_before","to_owned","try_from","try_into","type_id","vzip","All","Apply","DatabaseOperation","Info","Migrate","Migrator","Plan","PlanType","Revert","add_migration","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migrations","apply_all","apply_migration","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","default","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fmt","fmt","from","from","from","generate_migration_plan","into","into","into","list_applied_migrations","lock","lock","lock","lock","lock","migrations","migrations","migrations_mut","migrations_mut","new","revert_all","revert_migration","table_name","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","unlock","unlock","unlock","unlock","unlock","vzip","vzip","vzip","with_prefix","Operation","down","is_destructible","up"],"q":[[0,"sqlx_migrator"],[17,"sqlx_migrator::cli"],[38,"sqlx_migrator::error"],[66,"sqlx_migrator::error::Error"],[71,"sqlx_migrator::migration"],[97,"sqlx_migrator::migrator"],[183,"sqlx_migrator::operation"],[187,"clap_builder::builder::command"],[188,"core::fmt"],[189,"core::fmt"],[190,"clap_builder"],[191,"core::result"],[192,"clap_builder::util::id"],[193,"core::option"],[194,"alloc::boxed"],[195,"sqlx_core::pool"],[196,"sqlx_core::database"],[197,"core::any"],[198,"sqlx_core::error"],[199,"std::io::error"],[200,"core::error"],[201,"alloc::string"],[202,"sqlx_core::error"],[203,"core::hash"],[204,"alloc::vec"],[205,"core::future::future"],[206,"core::pin"],[207,"sqlx_core::any::database"],[208,"sqlx_postgres::database"],[209,"sqlx_sqlite::database"],[210,"sqlx_mysql::database"],[211,"std::collections::hash::set"],[212,"core::convert"]],"d":["Macro for implementing the <code>Migration</code> trait for the <code>Any</code>.","Macro for defining any SQL operations.","Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait","Macro for implementing the <code>Migration</code> trait for the …","Migrator module","Macro for implementing the <code>Migration</code> trait for the <code>MySql</code>.","Macro for defining mysql SQL operations.","Operation module","Macro for defining SQL operations.","Macro for implementing the <code>Migration</code> trait for the <code>Postgres</code>…","Macro for defining postgres SQL operations.","Macro for implementing the <code>Migration</code> trait for the <code>Sqlite</code>.","Macro for defining sqlite SQL operations.","","Macro for vector of box","Migration command for performing rust based sqlx migrations","","","","","","","","Returns the argument unchanged.","","","","Calls <code>U::from(self)</code>.","Parse <code>MigrationCommand</code> and run migration command line …","Run migration command line interface","","","","","","","Error when provided app name doesn’t exists","Error when migration name is only present but not app name","Error when applied migrations exists","Error when migration plan has applied replaces migrations …","Error when count of migrations is big than total number of …","Error enum to store different types of error","Error for failed to create migrations plan","Error for irreversible operation","Error when provided migration name doesn’t exists for app","Error when passed prefix is not alpha numeric","Error for pending migration present","Error type created from error raised by sqlx","Error type created from error raised by std input output","Error when unsupported database is used as any database","","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","Actual length of migration","Name of app","Name of app","Count passed in option","Name of migration","Migration struct created from sql table. Struct contains 4 …","Trait for migration","Migration app name. Can be name of folder or library where …","Return migration applied time","","","","","","","Returns the argument unchanged.","","","Return id value present on database","Calls <code>U::from(self)</code>.","Whether migration is atomic or not. By default it is true","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Replace certain migrations. If any one of listed migration …","Run before certain migration. This can be helpful in …","","","","","","Plan type used when listing all migration in chronological …","Plan type used when listing migrations which can be applied","Trait which is implemented for database for performing …","Info trait which implements some of database agnostic …","Migrate trait which migrate a database according to …","Migrator struct which store migrations graph and …","Struct which determine type of plan to use","Type of plan which needs to be generate","Plan type when listing migrations which can be reverted","Add single migration to migrator object","Add migration to migration db table","","","","","Add vector of migrations to Migrator object","Apply all migrations which are not applied till now","Apply given migration and add it to applied migration table","","","","","","","","Delete migration from migration db table","","","","","Drop migration table if migration table exists","","","","","Ensure migration table is created before running …","","","","","List all applied migrations from database as struct","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Generate migration plan according to plan. Returns a …","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","List all applied migrations. Returns a vector of migration","Lock database while doing migrations so no two migrations …","","","","","Return migrations","","Return mutable reference of migrations","","Create new plan using plan type, app name and migration …","Revert all applied migration from database","Revert provided migration and remove migration from table","Get name of table which is used for storing migrations …","","","","","","","","","","Unlock locked database","","","","","","","","Use prefix for migrator table name only ascii alpha …","Trait for operation","Down command to be executed during migration rollback. If …","Whether up operation is destructible or not. If operation …","Up command to be executed during migration apply"],"i":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,14,14,14,14,14,0,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,14,44,45,46,44,46,0,0,21,23,23,23,23,23,21,23,23,23,21,23,23,21,21,21,21,21,21,23,23,23,23,23,40,40,0,0,0,0,0,0,40,31,32,36,36,36,36,31,10,10,36,40,41,36,40,41,36,32,36,36,36,36,32,36,36,36,36,32,36,36,36,36,32,36,36,36,36,40,41,36,40,41,10,36,40,41,10,32,36,36,36,36,31,36,31,36,41,10,10,36,36,40,41,36,40,41,36,40,41,32,36,36,36,36,36,40,41,36,0,29,29,29],"f":"``````````````````{bb}0{ce{}{}}0{{}b}0{{df}h}{cc{}}{j{{n{dl}}}}0{{}{{Ab{A`}}}}5{{{Af{Ad}}{Ah{c}}}{{n{AjAl}}}An}{{d{Af{Ad}}{Ah{c}}}{{n{AjAl}}}An}{c{{n{e}}}{}{}}0{cB`{}}{{dj}{{n{Ajl}}}}0:``````````````::{{Alf}h}0{BbAl}{BdAl}:={Al{{Ab{Bf}}}}{cBh{}}776?```````{BjBl}{BnBl}{ce{}{}}0{BnBn}{{ce}Aj{}{}}{{BjBj}C`}{{Bn{Af{Bj}}}C`}{cc{}}{c{{Cb{Bn}}}Cd}{{Bjc}AjCf}{BnCh}8{BjC`};{Bj{{Cl{{Af{Cj}}}}}}{Bj{{Cl{{Af{Bj}}}}}}00;{c{{n{e}}}{}{}}0{cB`{}}=`````````{{Cn{Af{Bj}}}Aj}{{D`{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{{Dh{Df}}{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dj}}{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dl}}{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dn}}{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{Cn{Cl{{Af{Bj}}}}}Aj}{{Ad{Ah{c}}}{{Dd{{Af{Db}}}}}An}{{Ad{Af{Bj}}}{{Dd{{Af{Db}}}}}}{ce{}{}}00000{{}{{Dh{c}}}{}}97685{D`{{Dd{{Af{Db}}}}}}{{{Dh{Dl}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dn}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dj}}}{{Dd{{Af{Db}}}}}}{{{Dh{Df}}}{{Dd{{Af{Db}}}}}}4231043012{{E`f}h}{{Ebf}h}{cc{}}00{{AdEb}{{Dd{{Af{Db}}}}}}:::{Ad{{Dd{{Af{Db}}}}}}96578{Cn{{Ed{{Af{Bj}}}}}}{{{Dh{c}}}{{Ed{{Af{Bj}}}}}{}}10{{E`{Ab{Bh}}{Ab{Bh}}}{{n{EbAl}}}}{{Ad{Ah{c}}}{{Dd{{Af{Db}}}}}An}{{Ad{Af{Bj}}}{{Dd{{Af{Db}}}}}}{{{Dh{c}}}Bl{}}{c{{n{e}}}{}{}}00000{cB`{}}00{D`{{Dd{{Af{Db}}}}}}>?{{{Dh{Dn}}}{{Dd{{Af{Db}}}}}}{{{Dh{Dl}}}{{Dd{{Af{Db}}}}}}{ce{}{}}00{{{Dh{c}}e}{{n{{Dh{c}}Al}}}{}{{Ef{Bh}}}}`{Cj{{Dd{{Af{Db}}}}}}{CjC`}1","c":[],"p":[[5,"Command",187],[5,"MigrationCommand",17],[5,"Formatter",188],[8,"Result",188],[5,"ArgMatches",189],[8,"Error",190],[6,"Result",191],[5,"Id",192],[6,"Option",193],[10,"Migrate",97],[5,"Box",194],[5,"Pool",195],[1,"unit"],[6,"Error",38],[10,"Database",196],[5,"TypeId",197],[6,"Error",198],[5,"Error",199],[10,"Error",200],[5,"String",201],[10,"Migration",71],[1,"str"],[5,"AppliedMigrationSqlRow",71],[1,"bool"],[8,"Result",198],[10,"Row",202],[10,"Hasher",203],[1,"i32"],[10,"Operation",183],[5,"Vec",204],[10,"Info",97],[10,"DatabaseOperation",97],[10,"Future",205],[5,"Pin",206],[5,"Any",207],[5,"Migrator",97],[5,"Postgres",208],[5,"Sqlite",209],[5,"MySql",210],[6,"PlanType",97],[5,"Plan",97],[5,"HashSet",211],[10,"Into",212],[15,"CountGreater",66],[15,"AppNameNotExists",66],[15,"MigrationNameNotExists",66]],"b":[[54,"impl-Display-for-Error"],[55,"impl-Debug-for-Error"],[56,"impl-From%3CError%3E-for-Error"],[57,"impl-From%3CError%3E-for-Error"],[108,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[109,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[110,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[111,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[123,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[124,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[125,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[126,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[128,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[129,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[130,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[131,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[133,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[134,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[135,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[136,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[138,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[139,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[140,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[141,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[153,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[154,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[155,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[156,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"],[175,"impl-DatabaseOperation%3CAny%3E-for-Migrator%3CAny%3E"],[176,"impl-DatabaseOperation%3CPostgres%3E-for-Migrator%3CPostgres%3E"],[177,"impl-DatabaseOperation%3CMySql%3E-for-Migrator%3CMySql%3E"],[178,"impl-DatabaseOperation%3CSqlite%3E-for-Migrator%3CSqlite%3E"]]}]\
]'));
if (typeof exports !== 'undefined') exports.searchIndex = searchIndex;
else if (window.initSearch) window.initSearch(searchIndex);
