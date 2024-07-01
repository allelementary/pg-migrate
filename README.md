# PG Migrate

## About
Database migration tool for PostgreSQL written in Rust.

## Features
- Create migration
- Upgrade / Downgrade database
- Get current migration
- Get head migration
- Get migrations history

## Installation

To use PG Migrate, add it to your `Cargo.toml`:

```toml
[dependencies]
pg_migrate = "0.1.0"
```

Set up the database URL in the environment variable `DATABASE_URL`.

```bash
DATABASE_URL=postgresql://username:password@localhost/dbname
```

## CLI Usage

- Create migration:
```bash
cargo run --bin pg_migrate_cli -- new <migration filename>
```

- Upgrade / Downgrade:
There is multiple options to upgrade or downgrade the database:
  - Upgrade to the latest migration:
    ```bash
    cargo run --bin pg_migrate_cli -- upgrade head
    ```
  - Upgrade / Downgrade to a specific migration by migration id:
    ```bash
    cargo run --bin pg_migrate_cli -- upgrade/downgrade <migration-id>
    ```
  - Upgrade / Downgrade by number of migrations:
    ```bash
    cargo run --bin pg_migrate_cli -- upgrade/downgrade number <number>
    ```

- Get head:
```bash
cargo run --bin pg_migrate_cli -- head
```

- Get current migration:
```bash
cargo run --bin pg_migrate_cli -- current
```

- Get migrations history
```bash
cargo run --bin pg_migrate_cli -- history
```
