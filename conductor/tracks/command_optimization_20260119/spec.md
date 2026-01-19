# Specification: Otimizar Geração de Comandos One-Liner

## Context
Atualmente, o `ask` pode gerar comandos que abrangem múltiplas linhas ou que possuem aspas mal formatadas, o que causa falhas na execução ou colagem incorreta no terminal. Para melhorar a UX e a confiabilidade, precisamos instruir e forçar o modelo a gerar, sempre que possível, comandos de linha única (one-liners) usando operadores lógicos (`&&`, `;`) e garantir o escape correto de caracteres especiais.

## Goals
1.  **Priorizar One-Liners:** Ajustar os prompts do sistema para instruir explicitamente os modelos a preferirem comandos de linha única concatenados.
2.  **Robustez de Sintaxe:** Garantir que aspas (simples e duplas) e caracteres de escape sejam tratados corretamente para evitar erros de shell.
3.  **Fallback Seguro:** Se um comando *precisar* ser multi-linha (ex: um script complexo), ele deve ser formatado de maneira que o terminal aceite (ex: usando `\` para quebra de linha de forma segura ou blocos `heredoc` apropriados).

## Requirements

### Prompt Engineering
- Alterar o prompt do sistema (System Prompt) para incluir diretrizes estritas sobre formatação de comandos.
- Exemplo de instrução: "Provide shell commands as a single line using `&&` or `;` where appropriate. Ensure proper quoting for arguments containing spaces."

### Command Processing (Post-Processing)
- (Opcional) Implementar uma camada de validação ou sanitização no código Rust que detecte quebras de linha desnecessárias e tente "achatar" o comando antes de apresentar ao usuário, se seguro.

### Testing
- Criar casos de teste onde a entrada natural solicite múltiplas ações (ex: "create a directory named test and a file inside it named hello.txt") e verificar se a saída é um one-liner (ex: `mkdir test && touch test/hello.txt`).

## User Experience
- O usuário deve perceber uma taxa muito menor de erros de sintaxe ao executar comandos complexos.
- Copiar e colar a saída do `ask` deve funcionar "de primeira" na grande maioria dos terminais.
