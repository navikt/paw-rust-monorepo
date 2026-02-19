use paw_test::setup_test_db::setup_test_db;

#[tokio::test]
async fn test_db_migrations() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}
