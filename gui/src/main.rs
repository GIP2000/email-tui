mod app;
mod message_collection;
use anyhow::Result;
use app::App;

fn main() -> Result<()> {
    dotenv::dotenv()?;
    let mut app = App::new()?;

    loop {
        if app.render() {
            break;
        }
    }

    Ok(())
}
