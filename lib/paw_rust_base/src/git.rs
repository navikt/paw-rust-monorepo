pub fn commit_hash() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("dev-build")
}