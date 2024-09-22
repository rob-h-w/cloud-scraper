use crate::core::password::Password;
use log::trace;
use rpassword::prompt_password;
use tokio::fs;

static ROOT_PASSWORD_FILE: &str = "root_password.yaml";
static INPUT_ROOT_PASSWORD: &str = "Input root password: ";

pub async fn create_root_password() -> Result<(), String> {
    let root_password = prompt_password(INPUT_ROOT_PASSWORD)
        .map_err(|e| format!("Could not read password because of {:?}", e))?;

    save_root_password(&root_password).await?;

    Ok(())
}

pub async fn check_root_password(password: &str) -> Result<bool, String> {
    if !root_password_exists().await {
        return Ok(false);
    }

    let root_password = fs::read_to_string(ROOT_PASSWORD_FILE)
        .await
        .map_err(|e| format!("Could not read password because of {:?}", e))?;
    let root_password: Password = serde_yaml::from_str(&root_password)
        .map_err(|e| format!("Could not deserialize password because of {:?}", e))?;
    Ok(root_password.verify(password))
}

pub async fn root_password_exists() -> bool {
    fs::metadata(ROOT_PASSWORD_FILE).await.is_ok()
}

async fn save_root_password(password: &str) -> Result<(), String> {
    trace!("Hashing root password.");
    let password = Password::new(password, 16);

    trace!("Writing root password to file.");
    fs::write(
        ROOT_PASSWORD_FILE,
        serde_yaml::to_string(&password)
            .map_err(|e| format!("Could not serialize password because of {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Could not write password because of {:?}", e))?;

    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use lazy_static::lazy_static;
    use std::sync::{Mutex, MutexGuard};

    pub static TEST_PASSWORD: &str = "test";
    lazy_static! {
        pub static ref TEST_ROOT_PASSWORD_MUTEX: Mutex<()> = Mutex::new(());
    }

    pub struct CleanableTestFile<'a> {
        _guard: MutexGuard<'a, ()>,
    }

    impl CleanableTestFile<'_> {
        async fn new() -> Self {
            let lock = TEST_ROOT_PASSWORD_MUTEX
                .lock()
                .expect("Could not lock mutex.");
            save_root_password(TEST_PASSWORD)
                .await
                .expect("Could not save password.");
            Self { _guard: lock }
        }

        #[allow(dead_code)]
        fn drop(&self) {
            std::fs::remove_file(ROOT_PASSWORD_FILE).expect("Could not remove root password file.");
        }
    }

    pub async fn with_test_root_password_scope<'a>() -> CleanableTestFile<'a> {
        CleanableTestFile::new().await
    }

    mod save_root_password {
        use super::*;

        #[tokio::test]
        async fn saves_the_root_password_correctly() {
            let _lock = TEST_ROOT_PASSWORD_MUTEX
                .lock()
                .expect("Could not lock mutex.");

            save_root_password("test")
                .await
                .expect("Could not save password.");

            let file_text_content = fs::read_to_string(ROOT_PASSWORD_FILE)
                .await
                .expect("Could not read password.");

            let password: Password =
                serde_yaml::from_str(&file_text_content).expect("Could not deserialize password.");

            assert!(password.verify("test"));

            fs::remove_file(ROOT_PASSWORD_FILE)
                .await
                .expect("Could not remove file.");
        }
    }
}
