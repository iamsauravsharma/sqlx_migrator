var searchIndex = new Map(JSON.parse('[\
["sqlx_migrator",{"doc":"Library to create sqlx migration using rust code instead …","t":"QCCCQCQCQQEQFNNNNNNNNNNNNNNNNNNNNPPPPGPPPPPPPPPPPNNNNNNNNNNNNNNOOOOOFKMNNNNNNNNNNNNNMMMNNNNNNNKKKFFNMNNNNNNNNNNNNNMNNNNMNNNNMNNNNMNNNNNNNNNNMNNNNMNMNNNNNMNNNNNNNNMNNNNNNNKNNM","n":["any_migration","cli","error","migration","migration","migrator","mysql_migration","operation","postgres_migration","sqlite_migration","sqlx","vec_box","MigrationCommand","augment_args","augment_args_for_update","borrow","borrow_mut","command","command_for_update","fmt","from","from_arg_matches","from_arg_matches_mut","group_id","into","parse_and_run","run","try_from","try_into","type_id","update_from_arg_matches","update_from_arg_matches_mut","vzip","AppNameNotExists","AppliedMigrationExists","BothMigrationTypeApplied","CountGreater","Error","FailedToCreateMigrationPlan","IrreversibleOperation","MigrationNameNotExists","MigrationReplacedMultipleTimes","NonAsciiAlphaNumeric","ParentIsNotApplied","PendingMigrationPresent","ReplaceRunBeforeMisMatch","Sqlx","StdIo","UnsupportedDatabase","borrow","borrow_mut","fmt","fmt","from","from","from","into","source","to_string","try_from","try_into","type_id","vzip","actual_len","app","app","count","migration","AppliedMigrationSqlRow","Migration","app","applied_time","borrow","borrow_mut","clone","clone_into","eq","eq","from","from_row","hash","id","into","is_atomic","name","operations","parents","replaces","run_before","to_owned","try_from","try_into","type_id","vzip","DatabaseOperation","Info","Migrate","Migrator","Plan","add_migration","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migration_to_db_table","add_migrations","apply_all","apply_count","apply_name","borrow","borrow","borrow_mut","borrow_mut","default","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","delete_migration_from_db_table","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","drop_migration_table_if_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","ensure_migration_table_exists","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fetch_applied_migration_from_db","fmt","from","from","generate_migration_plan","into","into","lock","lock","lock","lock","lock","migrations","migrations","migrations_mut","migrations_mut","revert_all","revert_count","revert_name","run","state","state","table_name","try_from","try_from","try_into","try_into","type_id","type_id","unlock","unlock","unlock","unlock","unlock","vzip","vzip","with_prefix","Operation","down","is_destructible","up"],"q":[[0,"sqlx_migrator"],[12,"sqlx_migrator::cli"],[33,"sqlx_migrator::error"],[63,"sqlx_migrator::error::Error"],[68,"sqlx_migrator::migration"],[94,"sqlx_migrator::migrator"],[170,"sqlx_migrator::operation"],[174,"clap_builder::builder::command"],[175,"core::fmt"],[176,"core::fmt"],[177,"clap_builder"],[178,"core::result"],[179,"clap_builder::util::id"],[180,"core::option"],[181,"alloc::boxed"],[182,"core::any"],[183,"sqlx_core::error"],[184,"std::io::error"],[185,"core::error"],[186,"alloc::string"],[187,"sqlx_core::error"],[188,"core::hash"],[189,"alloc::vec"],[190,"core::future::future"],[191,"core::pin"],[192,"sqlx_core::any::database"],[193,"core::marker"],[194,"core::marker"],[195,"sqlx_mysql::database"],[196,"sqlx_sqlite::database"],[197,"core::default"],[198,"core::convert"]],"d":["Macro for implementing the <code>migration</code> macro for the <code>Any</code>.","Module for creating and running cli with help of migrator","Module for library error","Module defining migration trait To create own implement …","Macro for implementing the <code>Migration</code> trait for the …","Migrator module","Macro for implementing the <code>migration</code> macro for the <code>MySql</code>.","Operation module To create own operation implement trait …","Macro for implementing the <code>migration</code> macro for the <code>Postgres</code>…","Macro for implementing the <code>migration</code> macro for the <code>Sqlite</code>.","","Macro for vector of <code>Box</code>","Migration command for performing rust based sqlx migrations","","","","","","","","Returns the argument unchanged.","","","","Calls <code>U::from(self)</code>.","Parse <code>MigrationCommand</code> and run migration command line …","Run migration command line interface","","","","","","","Error when provided app name doesn’t exists","Error when applied migrations exists","Error when migration plan has applied replaces migrations …","Error when count of migrations is big than total number of …","Error enum to store different types of error","Error for failed to create migrations plan","Error for irreversible operation","Error when provided migration name doesn’t exists for app","Error when one migration is replaced by multiple times …","Error when passed prefix is not alpha numeric","Parent is not applied","Error for pending migration present","Error raised when order cannot be determine properly for …","Error type created from error raised by sqlx","Error type created from error raised by std input output","Error when unsupported database is used as any database","","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","Actual length of migration","Name of app","Name of app","Count passed in option","Name of migration","Migration struct created from sql table. Struct contains 4 …","Trait for migration","Migration app name. Can be name of folder or library where …","Return migration applied time","","","","","","","Returns the argument unchanged.","","","Return id value present on database","Calls <code>U::from(self)</code>.","Whether migration is atomic or not. By default it is …","Migration name. Can be file name without extension","Operation performed for migration (create, drop, etc.)","Parents of migration (migrations that should be applied …","Replace certain migrations. If any one of listed migration …","Run before(for applying)/after(for reverting) certain …","","","","","","Trait which is implemented for database for performing …","Info trait which implements some of database agnostic …","Migrate trait which migrate a database according to …","Migrator struct which store migrations graph and …","Struct which determine type of plan to use","Add single migration to migrator object","Add migration to migration db table","","","","","Add vector of migrations to Migrator object","Create new plan for apply all","Create new plan for apply count","Create new plan for apply for provided app and migration …","","","","","","Delete migration from migration db table","","","","","Drop migration table if migration table exists","","","","","Ensure migration table is created before running …","","","","","List all applied migrations from database as struct","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Generate migration plan according to plan. Returns a …","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Lock database while doing migrations so no two migrations …","","","","","Return migrations","","Return mutable reference of migrations","","Create new plan for revert all","Create new plan for revert count","Create new plan for revert for provided app and migration …","Run provided plan migrations","Return state used in migrator","","Get name of table which is used for storing migrations …","","","","","","","Unlock locked database","","","","","","","Use prefix for migrator table name only ascii alpha …","Trait for operation","Down command to be executed during migration rollback. If …","Whether up operation is destructible or not. If operation …","Up command to be executed during migration apply"],"i":[0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,13,13,13,13,0,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,13,44,45,46,44,46,0,0,19,21,21,21,21,21,19,21,21,21,19,21,21,19,19,19,19,19,19,21,21,21,21,21,0,0,0,0,0,29,30,34,34,34,34,29,40,40,40,34,40,34,40,34,30,34,34,34,34,30,34,34,34,34,30,34,34,34,34,30,34,34,34,34,40,34,40,10,34,40,30,34,34,34,34,29,34,29,34,40,40,40,10,29,34,34,34,40,34,40,34,40,30,34,34,34,34,34,40,34,0,27,27,27],"f":"`````````````{bb}0{ce{}{}}0{{}b}0{{df}h}{cc{}}{j{{n{dl}}}}0{{}{{Ab{A`}}}}5{{{Af{Ad}}}{{n{AhAj}}}}{{d{Af{Ad}}}{{n{AhAj}}}}{c{{n{e}}}{}{}}0{cAl{}}{{dj}{{n{Ahl}}}}0:````````````````::{{Ajf}h}0{AnAj}{B`Aj}:={Aj{{Ab{Bb}}}}{cBd{}}776?```````{BfBh}{BjBh}{ce{}{}}0{BjBj}{{ce}Ah{}{}}{{BfBf}Bl}{{Bj{Af{Bf}}}Bl}{cc{}}{c{{Bn{Bj}}}C`}{{Bfc}AhCb}{BjCd}8{BfBl};{Bf{{Ch{{Af{Cf}}}}}}{Bf{{Ch{{Af{Bf}}}}}}00;{c{{n{e}}}{}{}}0{cAl{}}=`````{{Cj{Af{Bf}}}Ah}{{Cl{Af{Bf}}}{{D`{{Af{Cn}}}}}}{{{Dd{Dbc}}{Af{Bf}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Djc}}{Af{Bf}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Dlc}}{Af{Bf}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Dnc}}{Af{Bf}}}{{D`{{Af{Cn}}}}}{DfDh}}{{Cj{Ch{{Af{Bf}}}}}Ah}{{}E`}{EbE`}{{Bh{Ab{Bd}}}E`}{ce{}{}}000{{}{{Dd{ce}}}{}Ed}:7689{Cl{{D`{{Af{Cn}}}}}}{{{Dd{Dbc}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Dnc}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Djc}}}{{D`{{Af{Cn}}}}}{DfDh}}{{{Dd{Dlc}}}{{D`{{Af{Cn}}}}}{DfDh}}4103240231{{E`f}h}{cc{}}0{{Ad{Ab{E`}}}{{D`{{Af{Cn}}}}}}9973645{Cj{{Ch{{Af{Bf}}}}}}{{{Dd{ce}}}{{Ch{{Af{Bf}}}}}{}{}}10>=<{{AdE`}{{D`{{Af{Cn}}}}}}{Cjc{}}{{{Dd{ce}}}e{}{}}{{{Dd{ce}}}Bh{}{}}{c{{n{e}}}{}{}}000{cAl{}}0?=><;{ce{}{}}0{{{Dd{ce}}g}{{n{{Dd{ce}}Aj}}}{}{}{{Ef{Bd}}}}`{{Cfc}{{D`{{Af{Cn}}}}}{DfDh}}{CfBl}1","c":[],"p":[[5,"Command",174],[5,"MigrationCommand",12],[5,"Formatter",175],[8,"Result",175],[5,"ArgMatches",176],[8,"Error",177],[6,"Result",178],[5,"Id",179],[6,"Option",180],[10,"Migrate",94],[5,"Box",181],[1,"unit"],[6,"Error",33],[5,"TypeId",182],[6,"Error",183],[5,"Error",184],[10,"Error",185],[5,"String",186],[10,"Migration",68],[1,"str"],[5,"AppliedMigrationSqlRow",68],[1,"bool"],[8,"Result",183],[10,"Row",187],[10,"Hasher",188],[1,"i32"],[10,"Operation",170],[5,"Vec",189],[10,"Info",94],[10,"DatabaseOperation",94],[10,"Future",190],[5,"Pin",191],[5,"Any",192],[5,"Migrator",94],[10,"Send",193],[10,"Sync",193],[5,"Postgres",194],[5,"MySql",195],[5,"Sqlite",196],[5,"Plan",94],[1,"usize"],[10,"Default",197],[10,"Into",198],[15,"CountGreater",63],[15,"AppNameNotExists",63],[15,"MigrationNameNotExists",63]],"b":[[51,"impl-Debug-for-Error"],[52,"impl-Display-for-Error"],[53,"impl-From%3CError%3E-for-Error"],[54,"impl-From%3CError%3E-for-Error"],[101,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[102,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[103,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[104,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[115,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[116,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[117,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[118,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[120,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[121,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[122,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[123,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[125,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[126,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[127,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[128,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[130,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[131,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[132,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[133,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[141,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"],[142,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[143,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[144,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[163,"impl-DatabaseOperation%3CSqlite,+State%3E-for-Migrator%3CSqlite,+State%3E"],[164,"impl-DatabaseOperation%3CAny,+State%3E-for-Migrator%3CAny,+State%3E"],[165,"impl-DatabaseOperation%3CPostgres,+State%3E-for-Migrator%3CPostgres,+State%3E"],[166,"impl-DatabaseOperation%3CMySql,+State%3E-for-Migrator%3CMySql,+State%3E"]]}]\
]'));
if (typeof exports !== 'undefined') exports.searchIndex = searchIndex;
else if (window.initSearch) window.initSearch(searchIndex);
