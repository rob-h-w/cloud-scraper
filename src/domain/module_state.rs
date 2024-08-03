use async_trait::async_trait;
use tokio::fs;

#[async_trait]
pub trait ModuleState {
    fn path() -> &'static str;
    async fn path_for<Module>() -> Result<String, std::io::Error>
    where
        Module: NamedModule,
    {
        let path = format!("{}/{}", Self::path(), Module::name());
        fs::create_dir_all(path.clone()).await?;
        Ok(path)
    }
}

pub trait NamedModule {
    fn name() -> &'static str;
}
