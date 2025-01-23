mod upload;

#[tokio::main]
async fn main() {
    println!("Running tests for upload enpoint..");
    upload::race_condition().await;
}
