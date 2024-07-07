# PG Migrate

## About
Database migration tool for PostgreSQL written in Rust

## Features
- Create migration
- Upgrade / Downgrade database
- Get current migration
- Get head migration
- Get migrations history

## Installation

```bash
cargo install pg_migrate
```

Set up the database URL and migrations directory in the environment variables `DATABASE_URL` and `MIGRATION_DIR`.

```bash
DATABASE_URL=postgresql://username:password@localhost/dbname
```

## CLI Usage

- Create migration:
```bash
pg_migrate_cli new <migration name>
```

- Upgrade / Downgrade:
There is multiple options to upgrade or downgrade the database:
  - Upgrade to the latest migration:
    ```bash
    pg_migrate_cli upgrade head
    ```
  - Upgrade / Downgrade to a specific migration by migration id:
    ```bash
    pg_migrate_cli upgrade/downgrade migration-id <migration-id>
    ```
  - Upgrade / Downgrade by number of migrations:
    ```bash
    pg_migrate_cli upgrade/downgrade number <number>
    ```

- Get head:
```bash
pg_migrate_cli head
```

- Get current migration:
```bash
pg_migrate_cli current
```

- Get migrations history
```bash
pg_migrate_cli history
```
