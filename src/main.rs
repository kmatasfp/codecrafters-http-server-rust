use errors::Result;
use server::Server;

mod errors;
mod server;
mod thread_pool;

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let server = Server::new("127.0.0.1:4221".to_string());
    server.listen()
}
