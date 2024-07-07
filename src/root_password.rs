use crate::core::password::Password;
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

async fn save_root_password(password: &str) -> Result<(), String> {
    println!("Hashing root password.");
    let password = Password::new(password, 16);

    println!("Writing root password to file.");
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
mod test {
    use super::*;

    mod save_root_password {
        use super::*;

        #[tokio::test]
        async fn saves_the_root_password_correctly() {
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
