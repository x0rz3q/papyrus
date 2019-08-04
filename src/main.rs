extern crate rand;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn random_slug() -> std::string::String {
	return thread_rng().sample_iter(&Alphanumeric).take(4).collect();
}

fn handle_connection(mut stream: TcpStream) {
	let mut buffer = [0; 51200];

	let size = match stream.read(&mut buffer) {
		Ok(size) => size,
		Err(_) => {
			println!("Cannot read from stream");
			return;
		}
	};

	let slug = random_slug();
	let mut file = match File::create(slug) {
		Ok(file) => file,
		Err(_) => {
			println!("Cannot create new file!");
			return;
		}
	};

	match file.write_all(&buffer[..size]) {
		Ok(_) => (),
		Err(_) => {
			println!("Cannot write to file");
		}
	};
}

fn main() {
	let listener = TcpListener::bind("127.0.0.1:9999").unwrap();

	for stream in listener.incoming() {
		let stream = match stream {
			Ok(stream) => stream,
			Err(_) => {
				println!("{}", "Cannot open stream");
				continue;
			}
		};

		handle_connection(stream);
	}
}
