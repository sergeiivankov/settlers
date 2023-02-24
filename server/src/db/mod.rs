mod migrations;
pub mod entities;

pub use self::migrations::Migrator;

// TODO: if https://github.com/SeaQL/sea-orm/pull/1475 will be accepted,
//       change migrations table name to just "migrations"