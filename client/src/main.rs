use client::{App};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wgpu=warn")); 

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .init();
    tracing::info!("Start du client");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);
    match event_loop.run_app(&mut app) {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Erreur sur le lancement de l'app : {e}");
        }
    }
}
