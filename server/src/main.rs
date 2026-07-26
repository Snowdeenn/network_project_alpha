use server::ServerApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = ServerApp::new()?;
    server.run();
    Ok(())
}