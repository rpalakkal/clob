mod order;
mod orderbook;
mod server;

#[tokio::main]
async fn main() {
    let state = server::SharedState::default();
    server::server(state).await;
}
