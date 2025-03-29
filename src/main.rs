pub mod core;
pub mod utils;

use crate::core::bundler_superaccount::{
    create_chunkers, fund_chunkers, get_chunkers, init_superaccount,
};
use crate::core::s3_client::init_s3_client;
#[tokio::main]

async fn main() {
    // let super_account = init_superaccount().await.unwrap();
    // println!("{:?}", super_account);
    // let chunkers = get_chunkers(super_account, None).await.unwrap();
    // let chunkers = fund_chunkers(chunkers).await.unwrap();
    // println!("{:?}", chunkers);

    init_s3_client().await.unwrap();
    println!("Hello, world!");
}
