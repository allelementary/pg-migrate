use clap::{Parser, Subcommand};
use pg_migrate::DbClient;
use dotenv::dotenv;
use std::env;

#[derive(Parser)]
#[command(name = "pg_migrate")]
#[command(about = "Database migration tool for PostgreSQL written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        name: String,
    },
    Upgrade {
        #[command(subcommand)]
        command: UpgradeSubcommands,
    },
    Downgrade {
        #[command(subcommand)]
        command: DowngradeSubcommands,
    },
    Head {},
    Current {},
    History {},
}

#[derive(Subcommand)]
enum UpgradeSubcommands {
    Head,
    MigrationId { id: String },
    Number { num: i32 },
}

#[derive(Subcommand)]
enum DowngradeSubcommands {
    MigrationId { id: String },
    Number { num: i32 },
}

fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let cli = Cli::parse();

    match &cli.command {
        Commands::New { name } => {
            let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
            db_client.create_new_migration(name).expect("Failed to create new migration");
        }
        Commands::Head {} => {
            let db_client = DbClient::new(&database_url).expect("Failed to initialize database");
            db_client.get_head().expect("Failed to get head");
        }
        Commands::Current {} => {
            let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
            db_client.get_current();
        }
        Commands::History {} => {
            let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
            db_client.get_history().expect("Failed to get history");
        }

        Commands::Upgrade { command } => match command {
            UpgradeSubcommands::Head => {
                let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
                db_client.run_migrations(true, true, None, None).expect("Failed to run migrations");
            }
            UpgradeSubcommands::MigrationId { id } => {
                let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
                db_client.run_migrations(true, false, Some(id), None).expect("Failed to run migrations");
            }
            UpgradeSubcommands::Number { num } => {
                let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
                db_client.run_migrations(true, false, None, Some(num)).expect("Failed to run migrations");
            }
        }

        Commands::Downgrade { command } => match command{
            DowngradeSubcommands::MigrationId { id } => {
                let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
                db_client.run_migrations(false, false, Some(id), None).expect("Failed to run migrations");
            }
            DowngradeSubcommands::Number { num } => {
                let mut db_client = DbClient::new(&database_url).expect("Failed to initialize database");
                db_client.run_migrations(false, false, None, Some(num)).expect("Failed to run migrations");
            }
        }
    }
}
