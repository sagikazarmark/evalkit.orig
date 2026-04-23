use std::env;

use evalkit_server::{RunStore, router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path = String::from("evalkit-server.sqlite");
    let mut listen = String::from("127.0.0.1:4000");
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => {
                if let Some(value) = args.next() {
                    db_path = value;
                }
            }
            "--listen" => {
                if let Some(value) = args.next() {
                    listen = value;
                }
            }
            _ => {}
        }
    }

    let store = RunStore::open(db_path)?;
    let app = router(store);
    let listener = tokio::net::TcpListener::bind(&listen).await?;

    println!("evalkit-server listening on {}", listener.local_addr()?);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
