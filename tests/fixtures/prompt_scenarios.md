# Baseline Prompt Scenarios

These scenarios are used to verify that the model generates robust, single-line commands (where possible) and handles escaping correctly.

## 1. Multi-step Operations
**Query:** "Create a directory named 'project', enter it, and initialize a new git repository"
**Risk:** Generates `cd` which doesn't persist in parent shell, or uses newlines.
**Target:** `mkdir -p project && cd project && git init` (Note: `cd` only affects subshell if executed directly, but `ask` executes via `sh -c`, so chaining is fine for the session, though `cd` won't affect user's shell unless sourced. For `ask`, we generally want commands that work in the one-off execution).

## 2. File Creation with Content
**Query:** "Create a python script named hello.py that prints 'Hello World'"
**Risk:** Uses interactive editors (`nano`) or complex heredocs that break line parsing.
**Target:** `echo "print('Hello World')" > hello.py` or `printf "print('Hello World')\n" > hello.py`

## 3. Conditional Logic
**Query:** "Delete node_modules if it exists"
**Risk:** `if [ -d node_modules ]; then rm -rf node_modules; fi` (Multiline)
**Target:** `[ -d node_modules ] && rm -rf node_modules` or `rm -rf node_modules` (since -f handles non-existence, but explicit check is safer/more pedagogical).

## 4. Loops / Batch Operations
**Query:** "Find all .txt files and move them to a 'docs' folder"
**Risk:** Multiline `for` loop.
**Target:** `mkdir -p docs && find . -maxdepth 1 -name "*.txt" -exec mv {} docs/ \;` or `mkdir -p docs && mv *.txt docs/ 2>/dev/null`

## 5. Complex Escaping
**Query:** "Add an alias to .zshrc: alias gcom='git commit -m \"wip\"'"
**Risk:** Nested quotes breaking the command string.
**Target:** `echo "alias gcom='git commit -m \"wip\"'" >> ~/.zshrc`

## 6. System Services
**Query:** "Install nginx and ensure it starts on boot"
**Risk:** Multiple commands on separate lines.
**Target:** `sudo apt install -y nginx && sudo systemctl enable --now nginx`
