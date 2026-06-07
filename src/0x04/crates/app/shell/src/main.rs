#![no_std]
#![no_main]

use lib::*;
use core::str::FromStr;

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
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "ps" => Ok(Command::Ps),
            "ls" => Ok(Command::ListApp),
            "hello" => Ok(Command::Hello),
            "test" => Ok(Command::Test),
            "clear" => Ok(Command::Clear),
            "exit" => Ok(Command::Exit),
            "help" => Ok(Command::Help),
            "time" => Ok(Command::Time),
            _ => Err(()),
        }        
    }
}

fn main() -> isize {
    println!("Welcome to Lvzw OS!");
    loop {
        print!("> ");
        let command = stdin().read_line();
        match command.parse::<Command>() {
            Ok(Command::Ps) => sys_stat(),
            Ok(Command::ListApp) => sys_list_app(),
            Ok(Command::Hello) => {
                let pid = sys_spawn("hello");
                sys_wait_pid(pid);
            }
            Ok(Command::Test) => {
                let pid = sys_spawn("test_app");
                sys_wait_pid(pid);
            }
            Ok(Command::Help) => {
                help()
            }
            Ok(Command::Exit) => {
                println!("Exit Shell...");
                break;
            }
            Ok(Command::Clear) => {
                print!("\x1b[2J\x1b[H");
            }
            Ok(Command::Time) => {
                let pid = sys_spawn("time");
                sys_wait_pid(pid);
            }
            Err(_) => println!("Unknown command, Please retry\nAvailable ")
        }
        
    }
    0
}

fn help() {
    println!("Developed by lvzw, whose student ID is 24353028");
    println!("Available Command: ps, ls, hello, test, clear, exit");
}

entry!(main);