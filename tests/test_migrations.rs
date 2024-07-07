extern crate gag;

#[macro_export]
macro_rules! assert_stdout_eq {
    ($test:expr, $expected:literal) => {{
        use gag::BufferRedirect;
        use std::io::Read;

        let mut buf = BufferRedirect::stdout().unwrap();

        $test;

        let mut output = String::new();
        buf.read_to_string(&mut output).unwrap();
        drop(buf);
        println!("Captured output: {:?}", output.trim());
        assert!(output.contains($expected), "Expected '{}' to be in '{}'", $expected, output);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use pg_migrate::setup::get_db_client;

    fn remove_test_migrations() {
        let migration_dir = env::var("MIGRATION_DIR").unwrap_or_else(|_| "migrations".to_string());
        for entry in fs::read_dir(&migration_dir).expect("Failed to read migration directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.is_file() && path.file_name().unwrap().to_str().unwrap().contains("test_migration") {
                fs::remove_file(path).expect("Failed to remove file");
            }
        }
    }

    #[test]
    fn test_create_new_migration() {
        let mut db_client = get_db_client().lock().unwrap();

        assert_stdout_eq!(
            db_client.create_new_migration("test_migration").expect("Failed to create migration"),
            "Created migration: "
        );
        remove_test_migrations();
    }

    #[test]
    fn test_upgrade_head() {
        let mut db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.run_migrations(true, true, None, None).expect("Failed to run migrations"),
            "Upgraded to head: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"
        );
        assert_stdout_eq!(
            db_client.run_migrations(false, false, None, Some(&2)).expect("Failed to downgrade"),
            "Downgraded to: \"None\""
        );
    }

    #[test]
    fn test_migrate_number() {
        let mut db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.run_migrations(true, false, None, Some(&1)).expect("Failed to upgrade"),
            "Upgraded to: f44e620f-60e0-4470-8904-44b4022b11a5"
        );
        assert_stdout_eq!(
            db_client.run_migrations(true, false, None, Some(&1)).expect("Failed to upgrade"),
            "Upgraded to: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"
        );
        assert_stdout_eq!(
            db_client.run_migrations(false, false, None, Some(&1)).expect("Failed to downgrade"),
            "Downgraded to: \"f44e620f-60e0-4470-8904-44b4022b11a5\""
        );
        assert_stdout_eq!(
            db_client.run_migrations(false, false, None, Some(&1)).expect("Failed to downgrade"),
            "Downgraded to: \"None\""
        );
    }

    #[test]
    fn test_migrate_id() {
        let mut db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.run_migrations(true, false, Some("f44e620f-60e0-4470-8904-44b4022b11a5"), None).expect("Failed to upgrade"),
            "Upgraded to target: f44e620f-60e0-4470-8904-44b4022b11a5"
        );
        assert_stdout_eq!(
            db_client.run_migrations(true, false, Some("622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"), None).expect("Failed to upgrade"),
            "Upgraded to target: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"
        );
        assert_stdout_eq!(
            db_client.run_migrations(false, false, Some("f44e620f-60e0-4470-8904-44b4022b11a5"), None).expect("Failed to downgrade"),
            "Downgraded to target: f44e620f-60e0-4470-8904-44b4022b11a5"
        );
        assert_stdout_eq!(
            db_client.run_migrations(false, false, None, Some(&1)).expect("Failed to downgrade"),
            "Downgraded to: \"None\""
        );
    }

    #[test]
    fn test_get_current() {
        let mut db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.get_current(),
            "Current: None"
        );
    }

    #[test]
    fn test_get_history() {
        let mut db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.run_migrations(true, true, None, None).expect("Failed to run migrations"),
            "Upgraded to head: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"
        );
        assert_stdout_eq!(
            db_client.get_history().expect("Failed to get history"),
            "f44e620f-60e0-4470-8904-44b4022b11a5 | add users\n622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd | add wallet"
        );
        let _ = db_client.run_migrations(false, false, None, Some(&2));
        assert_stdout_eq!(
            db_client.get_current(),
            "Current: None"
        );
    }

    #[test]
    fn test_get_head() {
        let db_client = get_db_client().lock().unwrap();
        assert_stdout_eq!(
            db_client.get_head().expect("Failed to get head"),
            "Head: 622511aa-d4ee-4ea7-a3c9-cd900bc2c2bd"
        );
    }
}
