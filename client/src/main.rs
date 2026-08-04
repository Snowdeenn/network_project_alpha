use client::App;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = App::new(&event_loop);
    match event_loop.run_app(&mut app) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Erreur sur l'event loop : {e}");
        }
    }
}