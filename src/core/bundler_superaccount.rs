use crate::utils::get_env::get_env_var;
use anyhow::Error;
use bundler::utils::core::super_account::{Chunker, SuperAccount};

pub async fn init_superaccount() -> Result<SuperAccount, Error> {
    let env_keystore_path = get_env_var("KEYSTORE_DIR")?;
    let env_pwd = get_env_var("SUPERACCOUNT_PWD")?;
    let env_funder_pk = get_env_var("SUPERACCOUNT_PK")?;
    Ok(SuperAccount::new()
        .keystore_path(env_keystore_path)
        .pwd(env_pwd)
        .funder(env_funder_pk))
}

pub async fn get_chunkers(
    super_account: SuperAccount,
    count: Option<u32>,
) -> Result<SuperAccount, Error> {
    let chunkers = super_account.load_chunkers(count).await?;
    Ok(chunkers)
}

pub async fn create_chunkers(
    super_account: SuperAccount,
    count: u32,
) -> Result<SuperAccount, Error> {
    let chunkers = super_account.create_chunkers(count).await?;
    Ok(chunkers)
}

pub async fn fund_chunkers(super_account: SuperAccount) -> Result<SuperAccount, Error> {
    let funded_chunkers = super_account.fund_chunkers().await?;
    Ok(funded_chunkers)
}
