fn main() {
  #[cfg(not(any(feature = "db_mysql", feature = "db_postgres", feature = "db_sqlite")))]
  exit({ eprintln!("Using one of `db_...` features is required"); 1 })
}