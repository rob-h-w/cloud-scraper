use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

#[async_trait]
pub trait ModuleState {
    fn path() -> &'static str;
    async fn path_for<Module>() -> Result<PathBuf, std::io::Error>
    where
        Module: NamedModule,
    {
        Self::path_for_name(Module::name()).await
    }

    async fn path_for_name(name: &str) -> Result<PathBuf, std::io::Error> {
        let path = PathBuf::from(Self::path()).join(name);
        fs::create_dir_all(&path).await?;
        Ok(path)
    }
}

pub trait NamedModule {
    fn name() -> &'static str;
}
