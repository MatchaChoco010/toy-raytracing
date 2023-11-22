use anyhow::Result;

mod app;

fn main() -> Result<()> {
    app::App::run()
}
