mod app;
mod ble;
mod speed;
mod tunnel;
mod uuid;

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    app::run();
}
