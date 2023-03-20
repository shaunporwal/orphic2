# Orphic
*A natural language shell interface for \*nix systems.*

---
### Overview
Orphic uses GPT to translate complex tasks into shell commands to be executed on the system. 


*Note: Orphic defaults to safe mode, and will not automatically execute commands without confirmation unless unsafe mode is specified.*

### Installation
* Make sure your system has rust and cargo.
* `cargo install orphic`
* Orphic requires the `OPENAI_API_KEY` environment variable to be set. You can generate one [here](https://openai.com/).

### Usage
Orphic is designed to be used like you would use any other CLI tool.

`$ orphic sort ~/Downloads into folders based on media type`

`$ orphic how strong is my network connection`

`$ orphic what version kernel am i running`

`$ orphic show me the name and size of all files larger than 8MB in ~/Downloads/` 

`$ orphic <do task that would otherwise require complex commands that you don't know off the top of your head>`

`-u` or `--unsafe` will execute commands without user verification.

`-4` or `--gpt4` will attempt to use GPT-4 instead of GPT-3.5-Turbo. Note that this will only work if your OpenAI account has access to the model.

`-i` or `--interpret` will describe the output of the task in natural language (note that this is generally very slow).
```
$ orphic -u -i how much disk space is available
You have 16GB available out of a total of 113GB on your main hard 
drive, which is mounted on the root directory. 
Other partitions and file systems are also listed with their 
respective usage percentages and mount points.
```

`-d` or `--debug` will display the raw GPT text along with the regular output, even in unsafe mode.
```
$ orphic -u -d count the lines of rust code in this directory excluding /target/.
{"command": "find . -name target -prune -o -name '*.rs' -type f -print0 | xargs -0 wc -l"}
61 ./src/prompts.rs
     219 ./src/main.rs
     280 total
```

`-r` or `--repl` will start Orphic in a REPL environment.
```
$ orphic -u -r
orphic> when did i last login
wtmp begins Sat Mar 18 14:55
orphic> quit
$
```
### Usage tips and observations 
Sometimes Orphic works. Sometimes it doesn't. GPT is inconsistent, and the prompts that I'm using leave a lot to be desired. Results seem to be better if you format your task as a command instead of a question ("list the currently open ports" instead of "what ports are currently open"). An error that often arises is that GPT will try to use commands or packages for a different OS/distribution, or will try to use tools that you don't currently have installed. A quick fix is to specify your OS if you think the task will require OS-specific tools, but I'm working on making Orphic more aware of which commands are at its disposal and which are not. 

### Contributing 
Pull requests welcome. If you use Orphic and get a good/interesting output, please send it to me. Likewise, if you get a really bad output, please also send it to me or open an issue. This system is very experimental and I'm still trying to figure out what works and what doesn't when it comes to prompts and configurations.

### License
[MIT](https://choosealicense.com/licenses/mit/)

Copyright (c) Will Savage, 2023

