# Implementation Plan - Otimizar Geração de Comandos One-Liner

## Phase 1: Analysis & Prompt Engineering

- [x] Task: Analisar os prompts atuais em `src/providers/*.rs` e `src/config/defaults.rs` para identificar onde as instruções de geração de comando são definidas. fa9ea2c
- [ ] Task: Criar um conjunto de "perguntas de teste" que historicamente geram comandos quebrados ou multi-linha para usar como baseline.
- [ ] Task: Refinar o System Prompt padrão para instruir explicitamente o uso de one-liners (`&&`, `;`) e escape correto de aspas.
    - [ ] Sub-task: Atualizar `DEFAULT_SYSTEM_PROMPT` ou equivalente.
    - [ ] Sub-task: Testar o novo prompt com o conjunto de baseline.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Analysis & Prompt Engineering' (Protocol in workflow.md)

## Phase 2: Implementation & Refinement

- [ ] Task: Implementar lógica de pós-processamento (se necessário) em `src/providers/mod.rs` ou `src/output/mod.rs` para sanitizar quebras de linha acidentais.
    - [ ] Sub-task: Write Tests: Criar testes unitários para a função de sanitização.
    - [ ] Sub-task: Implement Feature: Implementar a sanitização.
- [ ] Task: Verificar comportamento com diferentes shells (bash, zsh, fish) se aplicável.
- [ ] Task: Atualizar a documentação (se houver mudanças no comportamento visível ou configuração).
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Implementation & Refinement' (Protocol in workflow.md)

## Phase 3: Validation

- [ ] Task: Executar suite completa de testes de integração para garantir que nenhuma funcionalidade existente foi quebrada.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Validation' (Protocol in workflow.md)
