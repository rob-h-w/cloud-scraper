use std::collections::HashMap;
use std::future::Future;
use std::net::TcpListener;
use std::pin::Pin;
use std::process::{Child, Command};

use crate::integration::dependencies::headless_browser::HeadlessBrowser;

use lazy_static::lazy_static;
use std::sync::Mutex;

mod headless_browser;

lazy_static! {
    static ref PROCESSES: Mutex<HashMap<String, std::process::Child>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Debug)]
pub(crate) enum DependencyError {
    Missing(String),
    InitializationFailed(String),
}

type GetDependencyInitializerResult = Pin<Box<dyn Future<Output = Result<(), DependencyError>>>>;

trait Dependency {
    const DEPENDENCY_NAME: &'static str;
    async fn is_available() -> bool {
        Command::new("which")
            .arg(Self::DEPENDENCY_NAME)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    async fn initialize() -> Result<Option<Child>, DependencyError>;
}

pub(crate) async fn initialize_dependencies() -> Result<(), Vec<DependencyError>> {
    let mut errors: Vec<DependencyError> = vec![];

    match initialize::<HeadlessBrowser>().await {
        Ok(process) => {
            if let Some(process) = process {
                PROCESSES
                    .lock()
                    .unwrap()
                    .insert(HeadlessBrowser::DEPENDENCY_NAME.to_string(), process);
            }
        }
        Err(e) => errors.push(e),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) async fn teardown_dependencies() {
    PROCESSES
        .lock()
        .unwrap()
        .iter_mut()
        .for_each(|(name, process)| {
            log::info!("Killing process: {}", name);
            process
                .kill()
                .expect(format!("Failed to kill {name}").as_str());
        });
}

async fn initialize<T: Dependency>() -> Result<Option<Child>, DependencyError> {
    if PROCESSES.lock().unwrap().contains_key(T::DEPENDENCY_NAME) {
        return Ok(None);
    }

    if T::is_available().await {
        T::initialize().await
    } else {
        Err(DependencyError::Missing(T::DEPENDENCY_NAME.to_string()))
    }
}

fn is_port_in_use(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => false,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_in_use() {
        // Use a well-known port that is likely to be in use
        assert!(is_port_in_use(80));
    }

    #[test]
    fn test_port_not_in_use() {
        // Use a random high port that is likely not to be in use
        assert!(!is_port_in_use(65535));
    }

    struct Ls {}

    impl Dependency for Ls {
        const DEPENDENCY_NAME: &'static str = "ls";

        async fn initialize() -> Result<Option<Child>, DependencyError> {
            Ok(None)
        }
    }

    struct MissingDependency {}

    impl Dependency for MissingDependency {
        const DEPENDENCY_NAME: &'static str = "missing";

        async fn initialize() -> Result<Option<Child>, DependencyError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_initialize_when_it_exists() {
        let result = initialize::<Ls>().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_missing_dependency() {
        let result = initialize::<MissingDependency>().await;
        assert!(result.is_err());
    }
}
