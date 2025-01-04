use std::{env, path::PathBuf};

use errors::Result;
use server::Server;

mod errors;
mod server;
mod thread_pool;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Args {
    directory: Option<PathBuf>,
}

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let args = parse_args(env::args().collect());

    let server = Server::new("127.0.0.1:4221".to_string(), args);
    server.listen()
}

fn parse_args(args: Vec<String>) -> Args {
    let mut args_iter = args.iter().peekable();

    let mut maybe_directory: Option<PathBuf> = None;

    while let Some(arg) = args_iter.next() {
        if arg.starts_with("--directory") {
            if let Some(next_arg) = args_iter.peek() {
                maybe_directory = Some(PathBuf::from(next_arg));
            }
            break;
        }
    }

    Args {
        directory: maybe_directory,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_args_should_match_requested_params() {
        let test_cases = vec![
            (
                vec![
                    "foo".to_string(),
                    "--directory".to_string(),
                    "/tmp/path".to_string(),
                ],
                Args {
                    directory: Some(PathBuf::from("/tmp/path")),
                },
            ),
            (
                vec!["foo".to_string(), "--directory".to_string()],
                Args { directory: None },
            ),
        ];

        for (test_case, expected) in test_cases {
            assert_eq!(parse_args(test_case), expected)
        }
    }
}
