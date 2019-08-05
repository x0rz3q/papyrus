extern crate rand;
#[macro_use]
extern crate log;
extern crate env_logger;

use clap::{App, Arg};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::exit;
use threadpool::ThreadPool;

fn random_slug() -> std::string::String {
	return thread_rng().sample_iter(&Alphanumeric).take(4).collect();
}

fn handle_connection(mut stream: TcpStream, directory: String, domain: String) {
	let mut buffer = [0; 51200];

	let size = match stream.read(&mut buffer) {
		Ok(size) => size,
		Err(e) => {
			error!(
				"Cannot read data from stream with peer {}: {}",
				stream.peer_addr().unwrap(),
				e
			);
			return;
		}
	};

	let slug = random_slug();
	let path = Path::new(&directory).join(slug.clone());

	debug!("Slug is {}", slug);
	debug!("Upload path is {}", path.display());

	let mut file = match File::create(path.clone()) {
		Ok(file) => file,
		Err(e) => {
			error!("Cannot create new file {}: {}", path.display(), e);
			return;
		}
	};

	match file.write_all(&buffer[..size]) {
		Ok(_) => (),
		Err(e) => {
			error!("Cannot write to file {}: {}", path.display(), e);
			return;
		}
	};
	match stream.write(format!("{}/{}\n", domain, slug).as_bytes()) {
		Ok(_) => (),
		Err(e) => {
			error!("Cannot write to stream: {}", e);
		}
	};
}

fn main() {
	env_logger::init();

	let matches = App::new("papyrus")
		.version("0.1.0")
		.author("x0rz3q <jacob@x0rz3q.com>")
		.about("Terminal pastebin")
		.arg(
			Arg::with_name("port")
				.env("PAPYRUS_PORT")
				.short("p")
				.long("port")
				.help("TCP port number")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("host")
				.env("PAPYRUS_HOST")
				.short("h")
				.long("host")
				.help("Host bind address")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("output")
				.env("PAPYRUS_OUTPUT")
				.short("o")
				.long("output")
				.help("Output directory")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("threads")
				.env("PAPYRUS_THREADS")
				.short("-t")
				.long("threads")
				.help("Thread count")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("domain")
				.env("PAPYRUS_DOMAIN")
				.short("-d")
				.long("domain")
				.help("Domain name to be used")
				.takes_value(true),
		)
		.get_matches();

	let port = matches.value_of("port").unwrap_or("9999");
	let host = matches.value_of("host").unwrap_or("127.0.0.1");
	let output = matches
		.value_of("output")
		.unwrap_or("/var/lib/papyrus/uploads")
		.to_string();
	let domain = matches
		.value_of("domain")
		.unwrap_or("http://localhost")
		.to_string();
	let threads = match matches.value_of("threads").unwrap_or("4").parse::<usize>() {
		Ok(threads) => threads,
		Err(_) => {
			error!("Threads argument should be an integer");
			exit(1);
		}
	};

	info!("Opening socket {}:{}", host, port);
	info!("Storing pastes in {}", output);

	let pool = ThreadPool::new(threads);
	let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();
	for stream in listener.incoming() {
		debug!("Connection established");

		let stream = match stream {
			Ok(stream) => stream,
			Err(e) => {
				error!("Cannot open incoming stream: {}", e);
				continue;
			}
		};

		debug!("Connected to {}", stream.peer_addr().unwrap());

		let output = output.clone();
		let domain = domain.clone();
		pool.execute(move || {
			handle_connection(stream, output, domain);
		});
	}
}
