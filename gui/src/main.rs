mod app;
mod message_collection;
use app::App;

fn main() {
    dotenv::dotenv().unwrap();
    let mut app = App::new().unwrap();

    loop {
        if app.render().unwrap() {
            break;
        }
    }
}
