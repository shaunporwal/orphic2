#[allow(unused, dead_code)]

use async_openai::{
    types::{ 
        CreateChatCompletionRequestArgs,
        ChatCompletionRequestMessage,
        Role 
    },  Client
};
use serde_json::Value;
use substring::Substring;
use execute::{Execute, shell};
use clap::{command, Arg, ArgAction};
use serde_json::json;

use std::error::Error;
use std::process::Stdio;
use std::io::{self, Write};
use std::fmt;

pub mod prompts;

const GPT_35_TURBO: &'static str = "gpt-3.5-turbo";
const GPT_4: &'static str = "gpt-4";
const GPT_4_TURBO: &'static str = "gpt-4-turbo";

#[derive(Debug)]
struct UserAbort();

#[derive(Debug, Copy, Clone)]
struct Flags {
    repl:          bool,
    interpret:     bool,
    debug:         bool,
    unsafe_mode:   bool,
    model: &'static str
}

impl fmt::Display for UserAbort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "User aborted command")
    }
}
impl Error for UserAbort {}

fn get_prompt(key: &'static str) -> &'static str {
    assert!(prompts::PROMPTS[key].is_string());
    prompts::PROMPTS[key].as_str().unwrap()
}

fn try_extract(body: &String) -> Option<Value> {
    if body.find('{') == None || body.find('}') == None {
        return None;
    }

    let data = body.substring(body.find('{').unwrap(),body.rfind('}').unwrap()+1); 
    
    match serde_json::from_str(&data) {
        Ok(commands) => Some(commands),
        Err(e) => { println!("{}", e); None }
    }
}

async fn parse_command(client: &Client, body: &String) -> Result<Option<Value>, Box<dyn Error>> {
    match try_extract(body) {
        Some(commands) => Ok(Some(commands)),
        None => {
            match verify_json(client, body).await? {
                Some(body) => Ok(try_extract(&body)),
                None => Ok(None)
            }
        }
    }
}

async fn verify_json(client: &Client, input: &String) -> Result<Option<String>, Box<dyn Error>> {
    let history = vec![
        ChatCompletionRequestMessage {
            role: Role::System,
            content: String::from(get_prompt("json_verify_system")),
            name: None
        },
        ChatCompletionRequestMessage {
            role: Role::User,
            content: String::from(get_prompt("json_verify_user")) + input,
            name: None
        }
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model(GPT_35_TURBO)
        .messages(history)
        .build()?;

    let response = client.chat().create(request).await?;
    let body = (response.choices[0]).message.content.to_owned();
    
    return match body.trim() {
        "" => Ok(None),
        _ => Ok(Some(body))
    }
}

async fn interpret(client: &Client, task: &String, output: &String, flags: Flags) -> Result<String, Box<dyn Error>> {
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model(flags.model)
        .messages(vec![
            ChatCompletionRequestMessage {
                role: Role::System,
                content: String::from(get_prompt("interpreter_system")),
                name: None
            },
            ChatCompletionRequestMessage {
                role: Role::User,
                content: String::from(json!({"task": task, "output": output}).to_string()) + get_prompt("interpreter_user"),
                name: None
            },
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    Ok((response.choices[0]).message.content.to_owned())
}

async fn try_command(client: &Client, input: String, history: &mut Vec<ChatCompletionRequestMessage>, flags: Flags) -> Result<String, Box<dyn Error>> {
    history.push(ChatCompletionRequestMessage {
        role: Role::User,
        content: input + get_prompt("assistant_user"),
        name: None
    });

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model(flags.model)
        .messages((*history).clone())
        .build()?;

    let response = client.chat().create(request).await?;
    let body = (response.choices[0]).message.content.to_owned();

    return match parse_command(client, &body).await? {
        Some(commands) => {
            match commands["command"].as_str() {
                Some(command) => {
                    if !flags.unsafe_mode {
                        let mut input = String::new();
                        println!("{}", command);
                        print!("Execute? [Y/n] ");
                        io::stdout().flush()?;
                        io::stdin().read_line(&mut input)?;

                        match input.trim().to_lowercase().as_str() {
                            ""  | "y" | "yes" => {},
                            _ => return Err(Box::new(UserAbort()))
                        };
                    }

                    let mut shell = shell(command);
                    shell.stdout(if !flags.interpret {Stdio::inherit()} else {Stdio::piped()});
                    Ok(String::from_utf8(shell.execute_output()?.stdout)? + "\n")

                },
                None => Ok(body + "\n")
            }
        },
        None => Ok(body + "\n")
    }
}
    
async fn repl(client: &Client, flags: Flags) -> Result<(), Box<dyn Error>> {
    let mut history: Vec<ChatCompletionRequestMessage> = Vec::new();

    loop {
       let mut input = String::new();
       print!("orphic> ");
       io::stdout().flush()?;
       io::stdin().read_line(&mut input)?;
       match input.as_str().trim() {
            "quit" => break,
            task => {
                let res_maybe = try_command(client, String::from(task), &mut history, flags).await;
                match res_maybe {
                    Ok(res) => {
                        history.push(ChatCompletionRequestMessage {
                            role: Role::Assistant,
                            content: res.clone(),
                            name: None
                        });
                        
                        if flags.interpret {
                            println!("{}", interpret(&client, &(String::from(task.trim())), &res, flags).await?);
                        } else {
                            print!("{}", res.trim());
                        }
                    },
                    Err(error) => { 
                        if error.is::<UserAbort>() {
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }

            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .arg(Arg::new("task").action(ArgAction::Append))
        .arg(
            Arg::new("repl")
            .short('r')
            .long("repl")
            .action(ArgAction::SetTrue)
            .help("Start a REPL environment for orphic commands")
        )
        .arg(
            Arg::new("interpret")
            .short('i')
            .long("interpret")
            .action(ArgAction::SetTrue)
            .help("Interpret output into natural language")
        )
        .arg(
            Arg::new("debug")
            .short('d')
            .long("debug")
            .action(ArgAction::SetTrue)
            .help("Display raw GPT output")
        )
        .arg(
            Arg::new("unsafe")
            .short('u')
            .long("unsafe")
            .action(ArgAction::SetTrue)
            .help("Execute commands without confirmation")
        )
        .arg(
            Arg::new("model")
            .short('m')
            .long("model")
            .value_parser([GPT_35_TURBO, GPT_4, GPT_4_TURBO])
            .default_value(GPT_4_TURBO)
            .help("Specify the GPT model to use (default: gpt-4-turbo)")
        )
        .get_matches();

    let selected_model = matches.get_one::<String>("model").map(|s| s.as_str()).unwrap_or(GPT_4_TURBO);
    // Map the chosen model &str to one of our static constants so lifetimes line up
    let model_static: &'static str = match selected_model {
        GPT_35_TURBO => GPT_35_TURBO,
        GPT_4 => GPT_4,
        _ => GPT_4_TURBO, // default/fallback
    };

    let flags = Flags {
        repl:        matches.get_flag("repl"),
        interpret:   matches.get_flag("interpret"),
        debug:       matches.get_flag("debug"),
        unsafe_mode: matches.get_flag("unsafe"),
        model:       model_static,
    };

    // Display selected model
    println!("Using model: {}", flags.model);

    let client = Client::new();

    if flags.repl {
        repl(&client, flags).await?;
        return Ok(());
    }

    let task = matches
        .get_many::<String>("task")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();

    let mut history: Vec<ChatCompletionRequestMessage> = Vec::new();

    let res = try_command(&client, task.join(" "), &mut history, flags).await?;

    if flags.interpret {
        println!("{}", interpret(&client, &(task.join(" ")), &res, flags).await?);
    } else {
        print!("{}", res.trim());
    }

    Ok(())
}
