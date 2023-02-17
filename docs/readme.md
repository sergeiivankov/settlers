# Documentation

TODO:
- Describe build system, all features, build commands examples
- Describe server configuration, config file paramenters, replace parameters by environment variables, config file places (parents of current dir, system user config folder)

## Server features

- `db_mysql` - using MySQL database for store data

- `db_postgres` - using PostgreSQL database for store data

- `db_sqlite` - using SQLite database for store data

- `public_resources_caching` - enable public resources files strong caching, it means that if public resource added, updated or deleted, server restart requred to build new cache

- `secure_server` - enable HTTPS server, see `.env.example` for required parameters

**Important:** using one of `db_...` features is required! By default `db_sqlite` is enabled. To use other database pass to cargo build or run command next flags: "--no-default-features --features settlers-server/db_..."