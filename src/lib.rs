pub mod setup;
use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::string::String;
use postgres::{Client, NoTls, Error};
use chrono::Utc;
use std::fs::File;
use uuid::Uuid;
use std::env;
use dotenv::dotenv;

pub struct DbClient {
    client: Client,
}

impl DbClient {

    pub fn new(database_url: &str) -> Result<Self, Error> {
        let mut client = Client::connect(database_url, NoTls)?;
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                migration_id TEXT
        )")?;

        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS history (
                id SERIAL PRIMARY KEY,
                migration_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )")?;

        Ok(DbClient { client })
    }

    pub fn create_new_migration(&mut self, name: &str) -> io::Result<()> {
        let migrations_dir = PathBuf::from(DbClient::get_migration_dir());
        if !migrations_dir.exists() {
            fs::create_dir(&migrations_dir).unwrap();
        }

        let head = self._get_head();

        let timestamp = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();
        let uuid = Uuid::new_v4();
        let up_file = migrations_dir.join(format!("{}_{}_{}_up.sql", timestamp, uuid, name));
        let down_file = migrations_dir.join(format!("{}_{}_{}_down.sql", timestamp, uuid, name));

        {
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&up_file)
                .unwrap();
            writeln!(file, "-- SQL commands to upgrade").unwrap();
            writeln!(file, "-- Revision: {}", uuid).unwrap();
            writeln!(file, "-- Down Revision: {}", head.as_deref().unwrap_or("None"))?;
        }

        {
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&down_file)
                .unwrap();
            writeln!(file, "-- SQL commands to downgrade").unwrap();
            writeln!(file, "-- Revision: {}", uuid).unwrap();
            writeln!(file, "-- Down Revision: {}", head.as_deref().unwrap_or("None"))?;
        }

        println!("Created migration: {}_{}", timestamp, name);
        Ok(())
    }

    pub fn run_migrations(&mut self, upgrade: bool, head: bool, target: Option<&str>, count: Option<&i32>) -> Result<(), Error> {
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        if upgrade {
            self._sort_paths(&mut paths, true);
        } else {
            self._sort_paths(&mut paths, false);
        }

        if head {
            let _ = self._upgrade_head(&mut paths);
        } else if !target.is_none() {
            let _ = self._migrate_target(&mut paths, upgrade, target.unwrap());
        } else if count != None {
            let _ = self._migrate_count(&mut paths, upgrade, count.unwrap());
        }
        Ok(())
    }

    pub fn get_head(&self) -> Result<(), Error> {
        let migration_id = self._get_head();
        if migration_id.is_none() {
            println!("Head: {:?}", migration_id);
            return Ok(());
        } else {
            println!("Head: {}", migration_id.unwrap());
            Ok(())
        }
    }

    pub fn get_current(&mut self) {
        let migration_id = self._get_current();
        if migration_id.is_none() {
            println!("Current: {:?}", migration_id);
        } else {
            println!("Current: {}", migration_id.unwrap());
        }
    }

    pub fn get_history(&mut self) -> io::Result<()> {
        let result = self.client.query(
            "SELECT * FROM history ORDER BY ID",
            &[]
        ).expect("Failed to get history");
        let rows = result.iter();

        println!("             Migration ID            |  Name  ");
        println!("----------------------------------------------");
        for row in rows {
            let migration_id: String = row.get("migration_id");
            let name: String = row.get("name");
            println!("{} | {}", migration_id, name);
        }

        Ok(())
    }

    fn _upgrade_head(&mut self, paths: &mut Vec<PathBuf>) -> Result<(), Error> {
        let suffix = { "_up.sql" };
        let current = self._get_current();
        let head = self._get_head().unwrap();
        let mut running = false;
        for path in paths {
            if path.to_str().unwrap().ends_with(suffix) {
                if current.is_none() {
                    running = true;
                }
                if !current.is_none() && path.to_str().unwrap().contains(current.as_deref().unwrap()) {
                    running = true;
                } else if running {
                    let migration = fs::read_to_string(&path).unwrap();
                    self.client.batch_execute(&migration)?;

                    let (_, _, migration_id, migration_name) = self._get_migration_details(&path);

                    self._record_current(Some(migration_id.clone()))?;
                    self._save_history(&migration_id, &migration_name)?;
                    if path.to_str().unwrap().contains(&head) {
                        println!("Upgraded to head: {}", &head);
                        break;
                    }
                }
            }
        }
        if !running {
            println!("No migrations to run");
        }

        Ok(())
    }

    fn _migrate_target(&mut self, paths: &mut Vec<PathBuf>, upgrade: bool, target: &str) -> Result<(), Error> {
        let suffix = if upgrade { "_up.sql" } else { "_down.sql" };
        let direction = if upgrade { "Upgraded" } else { "Downgraded" };
        let current = self._get_current();

        let target_exists: bool = self._if_target_exists(target);
        if !target_exists {
            println!("Target migration does not exist");
            return Ok(());
        }

        let mut running = false;
        for path in paths {
            if path.to_str().unwrap().ends_with(suffix) {
                if current.is_none() || !upgrade && path.to_str().unwrap().contains(current.as_deref().unwrap()) {
                    running = true;
                }
                if !current.is_none() && path.to_str().unwrap().contains(current.as_deref().unwrap()) && upgrade {
                    running = true;
                } else if running {
                    if upgrade {
                        let migration = fs::read_to_string(&path).unwrap();
                        self.client.batch_execute(&migration)?;
                        let (_, _, migration_id, migration_name) = self._get_migration_details(&path);

                        self._record_current(Some(migration_id.clone()))?;
                        self._save_history(&migration_id.clone(), &migration_name)?;

                        if path.to_str().unwrap().contains(target) {
                            println!("{} to target: {}", direction, target);
                            break;
                        }
                    } else {
                        let down_migration_id: Option<String> = self._get_down_migration_id(path.to_str().unwrap());
                        if path.to_str().unwrap().contains(target) {
                            println!("{} to target: {}", direction, &target);
                            break;
                        }

                        let migration = fs::read_to_string(&path).unwrap();
                        self.client.batch_execute(&migration)?;
                        let (_, _, migration_id, _) = self._get_migration_details(&path);

                        let _ = self._record_current(down_migration_id.clone());
                        self._remove_from_history(&migration_id.clone())?;
                    }
                }
            }
        }
        if !running {
            println!("No migrations to run");
        }
        Ok(())
    }

    fn _migrate_count(&mut self, paths: &mut Vec<PathBuf>, upgrade: bool, count: &i32) -> Result<(), Error> {
        let suffix = if upgrade { "_up.sql" } else { "_down.sql" };
        let direction = if upgrade { "Upgraded" } else { "Downgraded" };
        let current = self._get_current();

        if !self._if_count_valid(count, upgrade) {
            println!("Invalid count");
            return Ok(());
        }

        let mut running = false;
        let mut counter = 0;
        for path in paths.iter() {
            if path.to_str().unwrap().ends_with(suffix) {
                if current.is_none() || !upgrade && path.to_str().unwrap().contains(current.as_deref().unwrap()) {
                    running = true;
                }
                if !current.is_none() && path.to_str().unwrap().contains(current.as_deref().unwrap()) && upgrade {
                    running = true;
                } else if running && counter < *count {
                    let sql = fs::read_to_string(&path).unwrap();
                    let _ = self.client.execute(&sql, &[]);
                    let (_, _, migration_id, migration_name) = self._get_migration_details(&path);
                    let down_migration_id: Option<String> = self._get_down_migration_id(path.to_str().unwrap());

                    if upgrade {
                        self._record_current(Some(migration_id.clone()))?;
                        self._save_history(&migration_id.clone(), &migration_name)?;
                    } else {
                        match down_migration_id.as_deref() {
                            Some("None") => {
                                let _ = self._record_current(None);
                            }
                            Some(_string) => {
                                let _ = self._record_current(down_migration_id.clone());
                                self._remove_from_history(&migration_id.clone())?;
                            }
                            None => {
                                let _ = self._record_current(None);
                            }
                        }
                    }
                    counter += 1;
                    if counter == *count {
                        if upgrade {
                            println!("{} to: {} {}", direction, &migration_id, &migration_name);
                        } else {
                            if !down_migration_id.is_none() {
                                println!("{} to: {:?}", direction, &down_migration_id.unwrap());
                            } else {
                                println!("{} to: {:?}", direction, &down_migration_id);
                            }
                        }

                        break;
                    }
                }
            }
        }
        if !running {
            println!("No migrations to run");
        }

        Ok(())
    }

    fn _get_head(&self) -> Option<String> {
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        self._sort_paths(&mut paths, false);

        if paths.len() > 0 {
            let filename = &paths[0];
            let split: Vec<&str> = filename.to_str().expect("REASON").split('_').collect();
            let migration_id = split[2];
            Some(String::from(migration_id))
        } else {
            None
        }
    }

    fn _get_current(&mut self) -> Option<String> {
        match self.client.query_opt(
            "SELECT migration_id FROM migrations ORDER BY id DESC LIMIT 1",
            &[]
        ) {
            Ok(Some(row)) => {
                let migration_id: Option<&str> = row.get("migration_id");
                migration_id.map(|s| s.to_string())
            }
            Ok(None) => None,
            Err(_) => None,
        }
    }

    fn _record_current(&mut self, migration_id: Option<String>) -> Result<(), Error> {
        self.client.execute(
            "DELETE FROM migrations",
            &[]
        )?;

        self.client.execute(
            "INSERT INTO migrations (migration_id) VALUES ($1)",
            &[&migration_id],
        )?;
        Ok(())
    }

    fn _save_history(&mut self, migration_id: &str, migration_name: &str) -> Result<(), Error> {
        self.client.execute(
            "INSERT INTO history (migration_id, name) VALUES ($1, $2) \
            ON CONFLICT (migration_id) DO NOTHING",
            &[&migration_id, &migration_name],
        )?;
        Ok(())
    }

    fn _remove_from_history(&mut self, migration_id: &str) -> Result<(), Error> {
        self.client.execute(
            "DELETE FROM history WHERE migration_id = $1",
            &[&migration_id],
        )?;
        Ok(())
    }

    fn _sort_paths(&self, paths: &mut Vec<PathBuf>, asc: bool) -> () {
        paths.sort_by(|a, b| {
            let a_metadata = fs::metadata(a).unwrap();
            let b_metadata = fs::metadata(b).unwrap();
            let a_time = a_metadata.created().unwrap();
            let b_time = b_metadata.created().unwrap();
            if asc {
                return a_time.cmp(&b_time);
            }
            b_time.cmp(&a_time)
        });
    }

    fn _if_target_exists(&self, target: &str) -> bool {
        let migrations_dir = "migrations";
        let paths: Vec<PathBuf> = fs::read_dir(migrations_dir).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        for path in paths {
            if path.to_str().unwrap().contains(target) {
                return true;
            }
        }
        false
    }

    fn _if_count_valid(&mut self, count: &i32, upgrade: bool) -> bool {
        let migrations_dir = "migrations";
        let mut paths: Vec<PathBuf> = fs::read_dir(migrations_dir).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let suffix = if upgrade { "_up.sql" } else { "_down.sql" };
        self._sort_paths(&mut paths, upgrade);
        let current = self._get_current();
        let mut running = false;
        let mut counter = 0;
        for path in paths.iter() {
            if path.to_str().unwrap().ends_with(suffix) {
                if upgrade && current.is_none() {
                    running = true;
                }
                if !current.is_none() && path.to_str().unwrap().contains(current.as_deref().unwrap()) {
                    running = true;
                    if !upgrade {
                        counter += 1;
                        if counter == *count {
                            return true;
                        }
                    }
                } else if running && counter < *count {
                    counter += 1;
                    if counter == *count {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn _get_migration_details(&self, path: &PathBuf) -> (String, String, String, String) {
        let mut split: Vec<&str> = path.to_str().expect("REASON").split('/').collect();
        split = split[1].split('_').collect();
        let date = split[0];
        let time = split[1];
        let migration_id = split[2];
        let _migration_name = &split[3..split.len()-1];
        let migration_name = _migration_name.join(" ");
        (date.to_string(), time.to_string(), migration_id.to_string(), migration_name)
    }

    fn _get_down_migration_id(&self, file_path: &str) -> Option<String> {
        let file = File::open(file_path).unwrap();
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();
            if line.starts_with("-- Down Revision: ") {
                let revision = line.trim_start_matches("-- Down Revision: ").to_string();
                return Some(revision)
            }
        }
        None
    }

    fn _get_history(&mut self) -> (Option<String>, Option<String>) {
        match self.client.query_opt(
            "SELECT * FROM history ORDER BY ID DESC LIMIT 1",
            &[]
        ) {
            Ok(Some(row)) => {
                let migration_id: Option<String> = row.get("migration_id");
                let name: Option<String> = row.get("name");
                (migration_id, name)
            }
            Ok(None) => (None, None),
            Err(_) => (None, None),
        }
    }

    pub fn get_migration_dir() -> String {
        dotenv().ok();
        env::var("MIGRATION_DIR").unwrap_or_else(|_| "migrations".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::get_db_client;

    #[test]
    fn test_get_head() {
        let db_client = get_db_client().lock().unwrap();
        let head = db_client._get_head();
        assert_eq!(head.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");
    }

    #[test]
    fn test_current_is_none() {
        let mut db_client = get_db_client().lock().unwrap();
        let current = db_client._get_current();
        assert_eq!(current.is_none(), true);
    }

    #[test]
    fn test_upgrade_head() {
        let mut db_client = get_db_client().lock().unwrap();

        let mut current = db_client._get_current();
        assert_eq!(current.is_none(), true);

        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, true);
        let _ = db_client._upgrade_head(&mut paths);

        current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        db_client._sort_paths(&mut paths, false);

        let _ = db_client._migrate_count(&mut paths, false, &2);
        current = db_client._get_current();
        assert_eq!(current.is_none(), true);
    }

    #[test]
    fn test_migrate_target() {
        let mut db_client = get_db_client().lock().unwrap();

        let mut current = db_client._get_current();
        assert_eq!(current.is_none(), true);

        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, true);

        let _ = db_client._migrate_target(&mut paths, true, "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        db_client._sort_paths(&mut paths, false);

        let _ = db_client._migrate_target(&mut paths, false, "f44e620f-60e0-4470-8904-44b4022b11a5");
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "f44e620f-60e0-4470-8904-44b4022b11a5");

        let _ = db_client._migrate_count(&mut paths, false, &1);
        current = db_client._get_current();
        assert_eq!(current.is_none(), true);

        db_client._sort_paths(&mut paths, true);

        let _ = db_client._migrate_target(&mut paths, true, "f44e620f-60e0-4470-8904-44b4022b11a5");
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "f44e620f-60e0-4470-8904-44b4022b11a5");

        let _ = db_client._migrate_target(&mut paths, true, "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        db_client._sort_paths(&mut paths, false);

        let _ = db_client._migrate_count(&mut paths, false, &2);
        current = db_client._get_current();
        assert_eq!(current.is_none(), true);
    }

    #[test]
    fn test_migrate_count() {
        let mut db_client = get_db_client().lock().unwrap();

        let mut current = db_client._get_current();
        assert_eq!(current.is_none(), true);

        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, true);

        let _ = db_client._migrate_count(&mut paths, true, &2);
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        db_client._sort_paths(&mut paths, false);

        let _ = db_client._migrate_count(&mut paths, false, &2);
        current = db_client._get_current();
        assert_eq!(current.is_none(), true);

        db_client._sort_paths(&mut paths, true);

        let _ = db_client._migrate_count(&mut paths, true, &1);
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "f44e620f-60e0-4470-8904-44b4022b11a5");

        let _ = db_client._migrate_count(&mut paths, true, &1);
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        db_client._sort_paths(&mut paths, false);

        let _ = db_client._migrate_count(&mut paths, false, &1);
        current = db_client._get_current();
        assert_eq!(current.unwrap(), "f44e620f-60e0-4470-8904-44b4022b11a5");

        let _ = db_client._migrate_count(&mut paths, false, &1);
        current = db_client._get_current();
        assert_eq!(current.is_none(), true);
    }

    #[test]
    fn test_record_current() {
        let mut db_client = get_db_client().lock().unwrap();

        let _ = db_client._record_current(Some(String::from("622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd")));
        let current = db_client._get_current();
        assert_eq!(current.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");

        let _ = db_client._record_current(None);
        let current = db_client._get_current();
        assert_eq!(current.is_none(), true);
    }

    #[test]
    fn test_save_and_remove_history() {
        let mut db_client = get_db_client().lock().unwrap();

        let (migration_id, _) = db_client._get_history();
        let _ = db_client._remove_from_history(migration_id.as_deref().unwrap());

        let _ = db_client._save_history(&"622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd", &"add_wallet");
        let (migration_id, migration_name) = db_client._get_history();

        assert_eq!(migration_id.unwrap(), "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");
        assert_eq!(migration_name.unwrap(), "add_wallet");

        let _ = db_client._remove_from_history("622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd");
        let (migration_id, migration_name) = db_client._get_history();

        assert_eq!(migration_id.is_none(), true);
        assert_eq!(migration_name.is_none(), true);
    }

    #[test]
    fn test_sort_paths_asc() {
        let db_client = get_db_client().lock().unwrap();
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, true);

        assert_eq!(paths[0].to_str().unwrap(), "migrations/2024-07-01_13:01:16_f44e620f-60e0-4470-8904-44b4022b11a5_add_users_up.sql");
    }

    #[test]
    fn test_sort_paths_desc() {
        let db_client = get_db_client().lock().unwrap();
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, false);

        assert_eq!(paths[0].to_str().unwrap(), "migrations/2024-07-02_10:49:20_622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd_add_wallet_down.sql");
    }

    #[test]
    fn test_if_target_exists() {
        let db_client = get_db_client().lock().unwrap();
        let target = "622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd";
        let exists = db_client._if_target_exists(target);
        assert_eq!(exists, true);

        let target = "aa2a8a12-4a44-4c66-8ebc-f7ca59a23cd3";
        let exists = db_client._if_target_exists(target);
        assert_eq!(exists, false);
    }

    #[test]
    fn test_if_count_valid() {
        let mut db_client = get_db_client().lock().unwrap();

        let is_valid = db_client._if_count_valid(&2, true);
        assert_eq!(is_valid, true);

        let is_valid = db_client._if_count_valid(&3, true);
        assert_eq!(is_valid, false);

        let is_valid = db_client._if_count_valid(&1, false);
        assert_eq!(is_valid, false);

        let is_valid = db_client._if_count_valid(&3, false);
        assert_eq!(is_valid, false);
    }

    #[test]
    fn test_get_migration_details() {
        let db_client = get_db_client().lock().unwrap();
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        db_client._sort_paths(&mut paths, true);

        let (migration_date, migration_time, migration_id, migration_name) = db_client._get_migration_details(&paths[0]);
        assert_eq!(migration_date, "2024-07-01");
        assert_eq!(migration_time, "13:01:16");
        assert_eq!(migration_id, "f44e620f-60e0-4470-8904-44b4022b11a5");
        assert_eq!(migration_name, "add users");
    }

    #[test]
    fn test_get_down_migration_id() {
        let db_client = get_db_client().lock().unwrap();
        let mut paths: Vec<PathBuf> = fs::read_dir(DbClient::get_migration_dir()).unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        db_client._sort_paths(&mut paths, true);

        let mut down_migration_id = db_client._get_down_migration_id(paths[0].to_str().unwrap());
        assert_eq!(down_migration_id.unwrap(), "None");

        down_migration_id = db_client._get_down_migration_id(paths[2].to_str().unwrap());
        assert_eq!(down_migration_id.unwrap(), "f44e620f-60e0-4470-8904-44b4022b11a5");
    }
}
