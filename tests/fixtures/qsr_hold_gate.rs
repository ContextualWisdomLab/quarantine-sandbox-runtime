use std::{
    env,
    io::{self, BufRead, Write},
    os::unix::process::CommandExt,
    process::{self, Command},
};

fn main() {
    let mut arguments = env::args_os();
    let _program_name = arguments.next();
    let Some(expected_release) = arguments.next() else {
        eprintln!("missing release token");
        process::exit(64);
    };
    let Some(consumer_program) = arguments.next() else {
        eprintln!("missing consumer program");
        process::exit(64);
    };
    let consumer_arguments = arguments.collect::<Vec<_>>();

    println!("QSR_GATE_READY");
    if io::stdout().flush().is_err() {
        process::exit(74);
    }

    let mut supplied_release = String::new();
    if io::stdin().lock().read_line(&mut supplied_release).is_err() {
        process::exit(74);
    }
    if supplied_release.trim_end().as_bytes() != expected_release.as_encoded_bytes() {
        eprintln!("release token mismatch");
        process::exit(77);
    }

    let error = Command::new(consumer_program)
        .args(consumer_arguments)
        .exec();
    eprintln!("consumer exec failed: {error}");
    process::exit(126);
}
