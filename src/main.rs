extern crate rand;
#[macro_use]
extern crate log;
extern crate env_logger;

use clap::{App, Arg};
use nix::unistd::{fork, ForkResult};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::prelude::*;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::exit;
use std::time::Duration;
use threadpool::ThreadPool;
use users::switch::{set_current_gid, set_current_uid};
use users::{get_current_uid, get_group_by_name, get_user_by_name};

fn random_slug() -> std::string::String {
	return thread_rng().sample_iter(&Alphanumeric).take(4).collect();
}

fn handle_connection(mut stream: TcpStream, directory: String, domain: String) {
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

	const MAX_SIZE: usize = 51200;
	let mut buffer = [0; 512];
	let mut total = 0;
	match stream.set_read_timeout(Some(Duration::from_secs(1))) {
		Ok(_) => (),
		Err(e) => {
			error!("Cannot set read timeout {}", e);
			return;
		}
	};

	loop {
		if total > MAX_SIZE {
			break;
		}

		let len = match stream.peek(&mut buffer) {
			Ok(len) => len,
			Err(_) => break,
		};

		if len == 0 {
			break;
		}

		let size = stream.read(&mut buffer).unwrap();
		match file.write_all(&buffer[..size]) {
			Ok(_) => (),
			Err(e) => {
				error!("Error writing file: {}", e);
				break;
			}
		};

		total += size;
		debug!("size: {}", size);
	}

	match stream.write(format!("{}/{}\n", domain, slug).as_bytes()) {
		Ok(_) => (),
		Err(e) => {
			error!("Cannot write to stream: {}", e);
			return;
		}
	};

	match stream.shutdown(Shutdown::Both) {
		Ok(_) => (),
		Err(e) => {
			error!("Cannot shutdown stream: {}", e);
		}
	}
}

fn is_root() -> bool {
	return get_current_uid() == 0;
}

fn switch_user(user: String) {
	if !is_root() {
		warn!(
			"Cannot switch to user {}: run as root to support user switching",
			user
		);
		return;
	}

	let user_id = match get_user_by_name(&user) {
		Some(user) => user.uid(),
		None => {
			warn!("User {} unknown", user);
			return;
		}
	};

	match set_current_uid(user_id) {
		Ok(_) => (),
		Err(e) => warn!("Cannot switch to user {}: {}", user, e),
	};
}

fn fork_process() {
	debug!("Forking process");
	match fork() {
		Ok(ForkResult::Parent { child, .. }) => {
			debug!("Child pid is {}", child);
			exit(1);
		}
		Ok(ForkResult::Child) => {
			return;
		}
		Err(_) => println!("Fork failed"),
	}
}

fn switch_group(group: String) {
	if !is_root() {
		warn!(
			"Cannot switch to group {}: run as root to support group switching",
			group
		);
		return;
	}

	let group_id = match get_group_by_name(&group) {
		Some(group) => group.gid(),
		None => {
			warn!("Group {} unknown", group);
			return;
		}
	};

	match set_current_gid(group_id) {
		Ok(_) => (),
		Err(e) => warn!("Cannot switch to group {}: {}", group, e),
	};
}

fn main() {
	env_logger::init();

	let matches = App::new("papyrus")
		.version("0.1.1")
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
		.arg(
			Arg::with_name("user")
				.env("PAPYRUS_USER")
				.short("-u")
				.long("user")
				.help("Papyrus user")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("group")
				.env("PAPYRUS_GROUP")
				.short("-g")
				.long("group")
				.help("Papyrus group")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("daemonize")
				.long("daemonize")
				.help("Daemonize papyrus"),
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

	match matches.value_of("group") {
		Some(group) => {
			debug!("Switching to group {}", group);
			switch_group(group.to_string());
		}
		None => (),
	};

	match matches.value_of("user") {
		Some(user) => {
			debug!("Switching to user {}", user);
			switch_user(user.to_string());
		}
		None => (),
	}

	match matches.occurrences_of("daemonize") {
		0 => (),
		_ => fork_process(),
	}

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
