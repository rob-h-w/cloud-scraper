use crate::core::password::Password;
use rpassword::prompt_password_from_bufread;
use std::io::{stdin, stdout};
use tokio::fs;

static ROOT_PASSWORD_FILE: &str = "root_password.yaml";

pub async fn create_root_password() -> Result<(), String> {
    let mut stdin = stdin().lock();
    let mut stdout = stdout().lock();
    let root_password = prompt_password_from_bufread(
        &mut stdin,
        &mut stdout,
        "Input root \
    password: ",
    )
    .map_err(|e| format!("Could not read password because of {:?}", e))?;

    fs::write(
        ROOT_PASSWORD_FILE,
        serde_yaml::to_string(&Password::new(root_password, 16))
            .map_err(|e| format!("Could not serialize password because of {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Could not write password because of {:?}", e))?;
    Ok(())
}
