# Plano de Desenvolvimento: CLI "ask"

## Objetivo

Criar um CLI em Rust chamado `ask` que permite interagir com modelos de IA usando texto livre sem necessidade de aspas. O usuário digita `ask sua pergunta aqui` e recebe comandos shell ou respostas conversacionais.

**Slogan do repositório**: "Ask anything in plain text, get commands or answers instantly. No quotes needed."

**Licença**: AGPLv3

## Diretrizes de Trabalho para o Agente Executor

### Modo de Trabalho

O agente deve trabalhar na base do projeto o máximo possível, implementando continuamente sem parar. Apenas no final, após implementar tudo que for possível, criar um arquivo `ROADMAP.md` com tarefas pendentes seguindo o formato especificado na seção "Formato do ROADMAP.md".

### Referência Obrigatória

Consultar constantemente o repositório `verseles/run` (https://github.com/verseles/run) como exemplo de:
- Arquitetura e estrutura de código
- Decisões de design
- Padrões de documentação
- Estilo de implementação

Visitar especificamente estes arquivos como templates obrigatórios:
- `https://raw.githubusercontent.com/verseles/run/refs/heads/main/.github/workflows/ci.yml` - Pipeline CI/CD
- `https://raw.githubusercontent.com/verseles/run/refs/heads/main/ADR.md` - Formato de Architecture Decision Records
- `https://raw.githubusercontent.com/verseles/run/refs/heads/main/install.sh` - Script de instalação Unix
- `https://raw.githubusercontent.com/verseles/run/refs/heads/main/install.ps1` - Script de instalação Windows

## Especificações Técnicas

### Stack Rust

**Crates principais**:
- `clap` (v4+) - CLI parsing com derive macros
- `serde` + `toml` - Config parsing
- `tokio` - Async runtime
- `reqwest` + `eventsource-stream` - HTTP client e SSE streaming
- `native_db` - Embedded database para contexto/histórico
- `colored` - Output colorizado no terminal
- `indicatif` - Spinners e progress indicators
- `dirs` - Detecção de diretórios cross-platform
- `self_update` - Auto-update do binário

### Parsing de Argumentos

O CLI deve aceitar flags antes ou depois do texto livre, sem necessidade de aspas:

```bash
ask --json qual é a capital da França? --verbose
ask --format markdown como fazer café --model gpt-4
ask qual o clima hoje? --json --api-key abc123
```

**Lógica de parsing**:
1. Percorrer todos os argumentos
2. Identificar flags (começam com `--` ou `-`)
3. Se flag requer valor (ex: `--model gpt-4`), o próximo arg é o valor
4. Se flag é booleana (ex: `--json`), marcar como true
5. Concatenar todos os args restantes como texto da pergunta

### Flags Principais

```
-c, --context         Usa/cria contexto do diretório atual
-x, --command         Força modo comando (bypassa detecção automática)
-y, --yes            Auto-executa comandos sem confirmação
-m, --model <MODEL>   Sobrescreve modelo configurado
-p, --provider <NAME> Sobrescreve provider configurado
    --json           Output em formato JSON
    --markdown       Output renderizado em Markdown (padrão para perguntas)
    --raw            Output texto puro sem formatação
    --no-color       Desativa colorização
    --no-follow      Desativa echo de resultado após execução
    --update         Verifica e instala atualizações
    --version        Mostra versão atual
```

### Sistema de Configuração

**Hierarquia de precedência** (maior para menor):
1. CLI arguments
2. Variáveis de ambiente (`ASK_*`)
3. `./ask.toml` ou `./.ask.toml` (projeto local)
4. `~/ask.toml` (home do usuário)
5. `~/.config/ask/config.toml` (padrão XDG)
6. Defaults hardcoded

**Variáveis de ambiente suportadas**:
```bash
ASK_PROVIDER=gemini
ASK_MODEL=gemini-3-flash-preview
ASK_GEMINI_API_KEY=...
ASK_OPENAI_API_KEY=sk-...
ASK_ANTHROPIC_API_KEY=sk-ant-...
ASK_STREAM=true
```

**Formato do config.toml**:

```toml
[default]
provider = "gemini"
model = "gemini-3-flash-preview"
stream = true

[providers.gemini]
api_key = "YOUR_API_KEY_HERE"
base_url = "https://generativelanguage.googleapis.com"  # opcional

[providers.openai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"  # opcional

[providers.openai_compatible]
api_key = "..."
base_url = "http://localhost:11434/v1"  # ex: Ollama
model = "llama3"

[providers.anthropic]
api_key = "sk-ant-..."
base_url = "https://api.anthropic.com"  # opcional

[behavior]
auto_execute = true  # auto-executa comandos seguros
confirm_destructive = true  # sempre pedir confirmação em destrutivos
timeout = 30  # segundos

[context]
max_age_minutes = 30  # TTL do contexto
max_messages = 20  # limite de mensagens no histórico
storage_path = "~/.local/share/ask/contexts"

[update]
auto_check = true  # verifica updates automaticamente
check_interval_hours = 24
channel = "stable"  # ou "beta", "nightly"

# Comandos customizados
[commands.cm]
system = "Generate concise git commit message based on diff"
type = "command"
inherit_flags = true
auto_execute = false

[commands.explain]
system = "Explain code in detail with examples"
inherit_flags = true

[commands.review]
system = "Code review with suggestions"
inherit_flags = false
provider = "claude"
model = "claude-3-5-sonnet"
```

### Comando `ask init`

**Comportamento**:
- Modo interativo que pergunta ao usuário as configurações
- Provider padrão sugerido: **Gemini**
- Modelo padrão: **`gemini-3-flash-preview`**
- Solicita API key interativamente
- Cria arquivo `~/ask.toml` com as configurações

**Idempotência**:
- Se executado novamente e arquivo já existe:
  - Comparar com template
  - Se for igual: avisar que já está configurado
  - Se for diferente: perguntar se deseja backup (`.bak`) e recriar
  - Opção de adicionar apenas campos faltantes (útil após updates)

**Exemplo de interação**:
```
$ ask init
? Select default provider: Gemini
? Enter Gemini API key: ***
? Enable streaming? Yes
✓ Created ~/ask.toml
```

### Detecção Automática de Intenção

Quando o usuário não usa flags explícitas (`-x` para comando), enviar prompt de classificação rápido:

**Sistema de classificação**:
```
Analise a entrada do usuário e classifique a intenção:

- COMMAND: deseja gerar/executar comandos shell
- QUESTION: pergunta conversacional/informacional
- CODE: deseja gerar código

Exemplos:
"list all docker containers" → COMMAND
"how does kubernetes work" → QUESTION  
"write python function to sort list" → CODE
```

**Implementação**:
- Usar modelo mais barato (ex: gpt-4o-mini ou gemini-flash) apenas para classificação
- Usar função calling/structured output para resposta confiável
- Após classificação, usar modelo configurado para resposta principal

### Execução de Comandos

**Detecção de comandos destrutivos**:
```rust
// Lista de padrões destrutivos
rm -rf, rm -r, rm /*
sudo (qualquer coisa)
dd, mkfs, fdisk, parted
chmod -R, chown -R
> /dev/*, > /etc/*
| sh, | bash, | zsh
wget | sh, curl | sh, curl | bash
kill -9, killall
```

**Comportamento de execução**:
- ✅ **Auto-executa**: comandos seguros (`ls`, `cd`, `grep`, `find`, `docker ps`, `git status`)
- ⚠️ **Pede confirmação**: comandos destrutivos detectados
- 🚀 **Flag `-y/--yes`**: força execução sem confirmação (qualquer comando)
- 📋 **Flag `--no-follow`**: executa sem echo de resultado

**Echo de resultado (padrão)**:
```bash
✓ Command generated: docker ps -a
[ Executing... ]
✓ Done: 3 containers listed (Success)
```

Cores:
- Verde: sucesso
- Vermelho: erro
- Amarelo: warning

**Se comando falhar**:
Oferecer continuação automática com contexto:
```
✗ Command failed with error: permission denied
? Try again with sudo? (Y/n)
```

### Sistema de Contexto (Opt-in)

**Ativação**: Flag `-c` ou `--context`

**Estrutura de armazenamento**:
```
~/.local/share/ask/
├── contexts.db (Native DB)
```

**Lógica**:
- **Sem `-c`**: stateless, cada pergunta é independente (comportamento padrão)
- **Com `-c`**: cria/usa contexto baseado no hash do `pwd` atual
- Contextos de diretórios diferentes não se misturam
- Limpeza automática conforme TTL configurado

**Comandos de gestão**:
```bash
ask -c "como instalar docker"        # primeira pergunta, cria contexto
ask -c "e no mac?"                   # continua contexto do diretório
ask -c --clear                       # limpa contexto atual
ask -c --history                     # mostra histórico do diretório
```

**Metadados de contexto**:
```json
{
  "pwd": "/home/user/projeto",
  "created_at": "2026-01-06T03:00:00Z",
  "last_used": "2026-01-06T04:00:00Z",
  "message_count": 5
}
```

### Integração com Providers

**APIs a implementar**:

**1. OpenAI (e compatíveis)**:
- Endpoint: `POST /v1/chat/completions`
- Streaming: SSE via `stream: true`
- Headers: `Authorization: Bearer {api_key}`

**2. Anthropic Claude**:
- Endpoint: `POST /v1/messages`
- Streaming: SSE via `stream: true`
- Headers: `x-api-key: {api_key}`, `anthropic-version: 2023-06-01`

**3. Google Gemini**:
- Endpoint: `POST /v1beta/models/{model}:generateContent`
- Streaming: SSE via `alt=sse`
- Query param: `key={api_key}`

**Timeout padrão**: 30 segundos (configurável)

### Streaming de Respostas

**Para perguntas (`QUESTION`, `CODE`)**:
- Streaming palavra por palavra
- Usar `stdout` + `flush()` após cada token
- Renderizar Markdown em tempo real

**Para comandos (`COMMAND`)**:
- Spinner/loading discreto usando `indicatif`
- Mostrar comando completo após geração
- Colorizar sintaxe do comando

**Implementação**:
```rust
use std::io::{self, Write};
use colored::*;

while let Some(chunk) = stream.next().await {
    let token = parse_token(chunk);
    print!("{}", token.bright_white());
    io::stdout().flush()?;
}
```

### Output Colorizado

**Padrão**: colorido por padrão

**Desativação**:
- Flag `--no-color`
- Variável de ambiente `NO_COLOR`
- Detecção automática de pipe/redirection

**Esquema de cores**:
```
✓ Sucesso: verde
✗ Erro: vermelho
⚠ Warning: amarelo
→ Prompt/Pergunta: ciano
📝 Info: azul
🔧 Comando: bright_white
```

**Biblioteca**: `colored` crate

### Suporte a Piping

Aceitar entrada via stdin:

```bash
cat package.json | ask "explain the code"
git diff | ask cm  # comando customizado
docker logs app | ask "find errors"
```

**Comportamento**:
- Detectar stdin com dados
- Incluir conteúdo no contexto do prompt
- Por padrão não usa contexto de diretório (stateless)
- Pode combinar com `-c` se necessário

### Auto-Update

**Biblioteca**: `self_update` crate

**Comandos**:
```bash
ask --update          # verifica e atualiza
ask --version         # mostra versão + aviso se update disponível
```

**Comportamento automático**:
- Verifica nova versão a cada 24h (configurável)
- Notificação discreta: `ℹ New version available: v2.1.0 (run 'ask --update')`
- Nunca atualiza silenciosamente sem permissão
- Busca releases no GitHub

**Config**:
```toml
[update]
auto_check = true
check_interval_hours = 24
channel = "stable"
```

### Comandos Customizados

Definidos no config.toml, podem:
- Ter system prompts específicos
- Herdar ou não flags globais
- Ter provider/modelo próprio
- Sobrescrever comportamento de auto_execute

**Uso**:
```bash
git diff | ask cm              # gera commit message
git diff | ask cm -y           # gera e executa
cat main.rs | ask explain -c   # explica com contexto
```

## Estrutura de Arquivos do Projeto

```
ask/
├── src/
│   ├── main.rs              # Entry point, setup CLI
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── parser.rs        # Argument parsing (flags + free text)
│   │   └── commands.rs      # Command definitions
│   ├── config/
│   │   ├── mod.rs
│   │   ├── loader.rs        # TOML config hierarchy
│   │   └── defaults.rs      # Default values
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── openai.rs        # OpenAI integration
│   │   ├── anthropic.rs     # Anthropic integration
│   │   ├── gemini.rs        # Google Gemini integration
│   │   └── traits.rs        # Provider trait
│   ├── context/
│   │   ├── mod.rs
│   │   ├── storage.rs       # Native DB operations
│   │   └── manager.rs       # Context lifecycle
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── safety.rs        # Destructive command detection
│   │   └── runner.rs        # Command execution
│   ├── output/
│   │   ├── mod.rs
│   │   ├── markdown.rs      # Terminal markdown rendering
│   │   ├── json.rs          # JSON output
│   │   └── colorize.rs      # Color schemes
│   └── update/
│       ├── mod.rs
│       └── checker.rs       # Version checking & updates
├── .github/
│   └── workflows/
│       ├── ci.yml           # Baseado em verseles/run
│       ├── release.yml      # Build e release automático
│       └── test.yml         # Tests e linting
├── install.sh               # Script instalação Unix (formato verseles/run)
├── install.ps1              # Script instalação Windows (formato verseles/run)
├── Cargo.toml
├── LICENSE                  # AGPLv3
├── README.md
├── ADR.md                   # Architecture Decision Records (formato verseles/run)
└── CODEBASE.md              # Documentação da estrutura do código
```

## GitHub Actions Pipelines

### `.github/workflows/ci.yml`

Seguir exatamente o padrão de `verseles/run/ci.yml`. Adaptar para:
- `cargo build`
- `cargo test --all-features`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`

### `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Upload to Release
        uses: svenstaro/upload-release-action@v2
        with:
          repo_token: ${{ secrets.GITHUB_TOKEN }}
          file: target/${{ matrix.target }}/release/ask${{ matrix.os == 'windows-latest' && '.exe' || '' }}
          asset_name: ask-${{ matrix.target }}${{ matrix.os == 'windows-latest' && '.exe' || '' }}
          tag: ${{ github.ref }}
```

### `.github/workflows/test.yml`

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Check formatting
        run: cargo fmt -- --check
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
```

## Scripts de Instalação

### `install.sh`

Seguir exatamente o formato de `verseles/run/install.sh`. Adaptar para:
- Detectar arquitetura (x86_64, aarch64)
- Detectar OS (Linux, macOS)
- Baixar binário correto do GitHub Releases
- Verificar checksum/hash
- Instalar em `/usr/local/bin/ask`
- Tornar executável

### `install.ps1`

Seguir exatamente o formato de `verseles/run/install.ps1`. Adaptar para:
- Detectar arquitetura Windows
- Baixar `.exe` do GitHub Releases
- Instalar em local apropriado do PATH
- Verificar hash

## Documentação

### `README.md`

Estrutura:
```markdown
# ask

> Ask anything in plain text, get commands or answers instantly. No quotes needed.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)

## Installation

### Unix/Linux/macOS
```bash
curl -fsSL https://raw.githubusercontent.com/verseles/ask/main/install.sh | sh
```

### Windows
```powershell
irm https://raw.githubusercontent.com/verseles/ask/main/install.ps1 | iex
```

## Quick Start

```bash
# Initialize configuration
ask init

# Ask anything without quotes
ask how to list docker containers
ask what is the capital of France

# Get commands
ask -x delete old log files

# Use context for follow-up questions
ask -c explain kubernetes
ask -c what about pods?

# Pipe input
git diff | ask cm
cat main.rs | ask explain
```

## Configuration

See `config.toml` examples in documentation.

## License

AGPLv3 - see [LICENSE](LICENSE)
```

### `ADR.md`

Seguir exatamente o formato de `verseles/run/ADR.md`. Criar decisões para:

**ADR-001**: Escolha de Native DB ao invés de SQLite
**ADR-002**: Parsing de argumentos sem aspas (flags antes ou depois do texto)
**ADR-003**: Contexto opt-in por padrão (-c flag)
**ADR-004**: TOML ao invés de YAML para configuração
**ADR-005**: Detecção automática de intenção (COMMAND vs QUESTION)
**ADR-006**: Gemini como provider padrão
**ADR-007**: Streaming com stdout+flush ao invés de ratatui

Cada ADR deve ter:
- Status (Proposto, Aceito, Rejeitado, Depreciado)
- Contexto
- Decisão
- Consequências

### `CODEBASE.md`

```markdown
# Codebase Structure

## Overview
CLI de IA que permite perguntas em texto livre sem aspas.

## Directory Structure
[Incluir árvore de diretórios detalhada do src/]

## Key Design Decisions
- See ADR.md for architectural decisions
- Context is opt-in (-c flag)
- Flags can come before or after text
- Native DB for context storage
- Streaming with stdout flush for smooth output

## Main Components

### CLI Parser (src/cli/)
Responsável por parsing de argumentos flexível.

### Config Loader (src/config/)
Carrega configurações com precedência: CLI > Env > Local > Home > Global.

### Providers (src/providers/)
Integrações com OpenAI, Anthropic, Gemini.

### Context Manager (src/context/)
Gerencia histórico usando Native DB, baseado em diretório.

### Executor (src/executor/)
Executa comandos com detecção de segurança.

### Output (src/output/)
Renderização de Markdown, JSON, colorização.
```

## Cargo.toml

```toml
[package]
name = "ask"
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0"
description = "Ask anything in plain text, get commands or answers instantly. No quotes needed."
repository = "https://github.com/verseles/ask"
authors = ["Verseles"]

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["stream", "json"] }
eventsource-stream = "0.5"
native_db = "0.8"
colored = "2"
indicatif = "0.17"
dirs = "5"
self_update = "0.39"
anyhow = "1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

## Architecture Decision Records (Principais)

### ADR-001: Native DB vs SQLite

**Status**: Aceito

**Contexto**: Precisamos armazenar histórico de conversas com baixa latência e integração type-safe com Rust.

**Decisão**: Usar Native DB por ser totalmente Rust, sem FFI, com mapamento automático de structs.

**Consequências**:
- ✅ Melhor integração com tipos Rust
- ✅ Menor overhead sem FFI
- ✅ Benchmarks comparáveis ou superiores ao SQLite
- ❌ Menos maduro que SQLite
- ❌ Menos ferramentas externas disponíveis

### ADR-002: Parsing Flexível de Argumentos

**Status**: Aceito

**Contexto**: Usuários não querem usar aspas para perguntas naturais.

**Decisão**: Flags podem vir antes ou depois do texto livre. Parser identifica flags e concatena o resto.

**Consequências**:
- ✅ UX muito melhor
- ✅ Comandos naturais sem friction
- ⚠️ Valores de flags devem vir imediatamente após a flag

### ADR-003: Contexto Opt-in

**Status**: Aceito

**Contexto**: Manter contexto pode ser confuso e consumir tokens desnecessariamente.

**Decisão**: Contexto é opt-in via flag `-c`. Por padrão, perguntas são stateless.

**Consequências**:
- ✅ Comportamento previsível por padrão
- ✅ Economia de tokens
- ✅ Usuário tem controle explícito

### ADR-004: TOML para Configuração

**Status**: Aceito

**Contexto**: Precisamos de formato de config legível e fácil de editar.

**Decisão**: TOML ao invés de YAML.

**Consequências**:
- ✅ Padrão no ecossistema Rust
- ✅ Menos "gotchas" que YAML
- ✅ Melhor para edição manual

### ADR-005: Detecção Automática de Intenção

**Status**: Aceito

**Contexto**: Usuários nem sempre sabem se querem comando ou resposta conversacional.

**Decisão**: Fazer classificação rápida com modelo leve antes de resposta principal.

**Consequências**:
- ✅ UX mais inteligente
- ✅ Usuário não precisa usar flags sempre
- ⚠️ Custo extra de uma requisição pequena

### ADR-006: Gemini como Padrão

**Status**: Aceito

**Contexto**: Escolher provider e modelo padrão para `ask init`.

**Decisão**: Gemini como provider padrão com modelo `gemini-3-flash-preview`.

**Consequências**:
- ✅ Modelo rápido e eficiente
- ✅ API key gratuita disponível para testes
- ⚠️ Usuários podem preferir outros providers

### ADR-007: Streaming Simples

**Status**: Aceito

**Contexto**: Usuários querem output suave como ChatGPT.

**Decisão**: Usar `stdout + flush()` ao invés de ratatui para streaming.

**Consequências**:
- ✅ Implementação muito mais simples
- ✅ Menor peso no binário
- ✅ Output integrável com pipes
- ❌ Sem TUI complexa (não necessária para o caso de uso)

## Formato do ROADMAP.md

Apenas criar este arquivo no final, após implementar o máximo possível. Usar formato:

```markdown
---
feature: "CLI ask - Initial Implementation"
spec: |
  AI-powered CLI that accepts plain text questions without quotes.
---

## Task List

### Feature 1: Core Infrastructure

Description: Basic setup, config loading, CLI parsing

- [x] 1.01 Setup Rust project with Cargo.toml
- [x] 1.02 Implement flexible argument parser
- [~] 1.03 Config loader with TOML hierarchy

### Feature 2: Provider Integrations

Description: Integrate OpenAI, Anthropic, Gemini APIs

- [ ] 2.01 OpenAI integration with streaming
- [ ] 2.02 Anthropic integration
- [ ] 2.03 Gemini integration (default)

### Feature 3: Context System

Description: Native DB storage for conversation history

- [/] 3.01 Native DB setup
- [ ] 3.02 Context manager with TTL
- [ ] 3.03 History commands

### Feature 4: Command Execution

Description: Safe command detection and execution

- [ ] 4.01 Safety detector for destructive commands
- [ ] 4.02 Command executor with follow-up echo
- [ ] 4.03 Confirmation prompts

### Feature 5: Advanced Features

Description: Streaming, auto-update, custom commands

- [ ] 5.01 SSE streaming implementation
- [ ] 5.02 Auto-update with self_update crate
- [ ] 5.03 Custom commands from config
- [ ] 5.04 Piping support (stdin)

### Feature 6: Documentation & CI/CD

Description: Documentation, install scripts, GitHub Actions

- [ ] 6.01 README.md
- [ ] 6.02 ADR.md with all decisions
- [ ] 6.03 CODEBASE.md
- [ ] 6.04 install.sh (Unix)
- [ ] 6.05 install.ps1 (Windows)
- [ ] 6.06 GitHub Actions CI/CD (following verseles/run pattern)
```

### Legenda

- [x] Completo
- [~] Em progresso recente
- [/] Parcialmente implementado mas não funcional (pode estar bloqueado)
- [ ] Não iniciado

### Regras Importantes

- Marcar [x] apenas quando completamente funcional
- Se tarefa está bloqueada por outra, marcar [/]
- Manter atualizado conforme progresso
