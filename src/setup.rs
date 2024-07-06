use std::env;
use postgres::{Client, NoTls, Error};
use crate::DbClient;
use dotenv::dotenv;
use std::sync::{Mutex, Once};

static INIT: Once = Once::new();
static mut DB_CLIENT: Option<Mutex<DbClient>> = None;

fn setup_test_database() -> Result<(), Error> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let test_database_name = env::var("TEST_DATABASE_NAME").expect("TEST_DATABASE_NAME must be set");
    let mut client = Client::connect(&database_url, NoTls)?;
    println!("Database URL: {}", &database_url);
    println!("Test database name: {}", &test_database_name);
    client.batch_execute(&format!("DROP DATABASE IF EXISTS {};", test_database_name))?;
    client.batch_execute(&format!("CREATE DATABASE {};", test_database_name))?;
    Ok(())
}

pub fn setup() {
    println!("Setting up tests");
    setup_test_database().expect("Failed to set up test database");
}

pub fn get_db_client() -> &'static Mutex<DbClient> {
    unsafe {
        INIT.call_once(|| {
            setup();
            let test_database_url = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
            let db_client = DbClient::new(&test_database_url).expect("Failed to initialize database");
            DB_CLIENT = Some(Mutex::new(db_client));
        });
        DB_CLIENT.as_ref().expect("DB Client not initialized")
    }
}
