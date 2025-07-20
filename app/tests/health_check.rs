mod common;

use common::{cleanup_test_database, create_test_db};
use ocloud::api::ApiClient;

#[tokio::test]
async fn health_check_works() {
    let db_pool = create_test_db().await;
    let client = ApiClient::new_local(db_pool.clone()).await;

    let result = client.health().await;

    assert!(result.is_ok());

    cleanup_test_database(db_pool).await;
}
