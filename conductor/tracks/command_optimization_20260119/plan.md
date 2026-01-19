# Implementation Plan - Otimizar Geração de Comandos One-Liner

## Phase 1: Analysis & Prompt Engineering [checkpoint: ecb1207]

- [x] Task: Analisar os prompts atuais em `src/providers/*.rs` e `src/config/defaults.rs` para identificar onde as instruções de geração de comando são definidas. fa9ea2c
- [x] Task: Criar um conjunto de "perguntas de teste" que historicamente geram comandos quebrados ou multi-linha para usar como baseline. 560e90c
- [x] Task: Refinar o System Prompt padrão para instruir explicitamente o uso de one-liners (`&&`, `;`) e escape correto de aspas. 39e1443
    - [x] Sub-task: Atualizar `DEFAULT_SYSTEM_PROMPT` ou equivalente.
    - [x] Sub-task: Testar o novo prompt com o conjunto de baseline.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Analysis & Prompt Engineering' (Protocol in workflow.md) ecb1207

## Phase 2: Implementation & Refinement

- [x] Task: Implementar lógica de pós-processamento (se necessário) em `src/providers/mod.rs` ou `src/output/mod.rs` para sanitizar quebras de linha acidentais. c22a94a
    - [x] Sub-task: Write Tests: Criar testes unitários para a função de sanitização.
    - [x] Sub-task: Implement Feature: Implementar a sanitização.
- [ ] Task: Verificar comportamento com diferentes shells (bash, zsh, fish) se aplicável.
- [ ] Task: Atualizar a documentação (se houver mudanças no comportamento visível ou configuração).
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Implementation & Refinement' (Protocol in workflow.md)

## Phase 3: Validation

- [ ] Task: Executar suite completa de testes de integração para garantir que nenhuma funcionalidade existente foi quebrada.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Validation' (Protocol in workflow.md)
