#![no_std]
#![no_main]

use core::str::FromStr;
use lib::*;

extern crate lib;

enum Command {
    Ps,
    ListApp,
    Hello,
    Test,
    Help,
    Clear,
    Exit,
    Time,
    Counter,
    Mq,
    Dinner,
    Ls,
    Shell,
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "ps" => Ok(Command::Ps),
            "lsapp" => Ok(Command::ListApp),
            "hello" => Ok(Command::Hello),
            "test" => Ok(Command::Test),
            "clear" => Ok(Command::Clear),
            "exit" => Ok(Command::Exit),
            "help" => Ok(Command::Help),
            "time" => Ok(Command::Time),
            "counter" => Ok(Command::Counter),
            "mq" => Ok(Command::Mq),
            "dinner" => Ok(Command::Dinner),
            "ls" => Ok(Command::Ls),
            "sh" => Ok(Command::Shell),
            _ => Err(()),
        }
    }
}

fn main() -> isize {
    println!("Welcome to Lvzw OS!");
    loop {
        print!("> ");
        let command = stdin().read_line();
        let command = command.trim();

        if cat(command) {
            continue;
        }

        match command.parse::<Command>() {
            Ok(Command::Ps) => sys_stat(),
            Ok(Command::ListApp) => sys_list_app(),
            Ok(Command::Hello) => spawn_and_wait("hello"),
            Ok(Command::Test) => spawn_and_wait("test"),
            Ok(Command::Help) => help(),
            Ok(Command::Exit) => {
                println!("Exit Shell...");
                break;
            }
            Ok(Command::Clear) => print!("\x1b[2J\x1b[H"),
            Ok(Command::Time) => spawn_and_wait("time"),
            Ok(Command::Counter) => spawn_and_wait("counter"),
            Ok(Command::Mq) => spawn_and_wait("mq"),
            Ok(Command::Dinner) => spawn_and_wait("dinner"),
            Ok(Command::Ls) => {
                if !sys_list_dir("APP") {
                    println!("no such file or directory");
                }
            }
            Ok(Command::Shell) => spawn_and_wait("sh"),
            Err(_) => println!(
                "Unknown command, Please retry\nAvailable command: ps, ls, cat, hello, test, clear, sh, time, exit"
            ),
        }
    }
    0
}

#[inline(always)]
fn help() {
    println!("Developed by lvzw, whose student ID is 24353028");
    println!("Available Command: ps, ls, cat, hello, test, clear, sh, time, exit");
}

#[inline(always)]
fn spawn_and_wait(path: &str) {
    let pid = sys_spawn(path);
    sys_wait_pid(pid);
}

entry!(main);
