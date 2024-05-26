use std::io::Read;
use std::process::{Child, Command};

use tokio::time::sleep;

use crate::integration::dependencies::{is_port_in_use, Dependency, DependencyError};

pub(crate) struct HeadlessBrowser {}

const BUF_SIZE: usize = 1024;
const SUCCESS_MESSAGE: &str = "ChromeDriver was started successfully.";
const RETRIES: usize = 5;
const SLEEP_TIME: std::time::Duration = std::time::Duration::from_secs(1);
const PORT: u16 = 9515;

impl Dependency for HeadlessBrowser {
    const DEPENDENCY_NAME: &'static str = "chromedriver";
    async fn initialize() -> Result<Option<Child>, DependencyError> {
        if is_port_in_use(PORT) {
            return Ok(None);
        }

        let mut process = Command::new(Self::DEPENDENCY_NAME)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                DependencyError::InitializationFailed(format!(
                    "Failed to start {}: {:?}",
                    Self::DEPENDENCY_NAME,
                    e
                ))
            })?;

        let mut buffer = [0; BUF_SIZE];
        let mut message = String::new();
        let mut stdout = process.stdout.take().unwrap();
        let mut attempt: usize = 0;

        while attempt < RETRIES {
            if attempt > 0 {
                sleep(SLEEP_TIME).await;
            }

            stdout
                .read(&mut buffer)
                .map_err(|e| DependencyError::InitializationFailed(format!("{:?}", e)))?;

            if let Ok(message_piece) = String::from_utf8(buffer.to_vec()) {
                message.push_str(&message_piece);
            }

            if message.contains(SUCCESS_MESSAGE) {
                return Ok(Some(process));
            }

            attempt += 1;
        }

        Err(DependencyError::InitializationFailed(format!(
            "Failed to start {}: {}",
            Self::DEPENDENCY_NAME,
            message
        )))
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;

    #[tokio::test]
    async fn test_initialize() {
        let result = HeadlessBrowser::initialize().await;
        assert!(result.is_ok());

        let process = result.unwrap();
        if let Some(mut process) = process {
            assert_ok!(process.kill());
        }
    }
}
