# Initial Concept

A CLI tool that lets you interact with AI models using natural language, without the need for quotes around your questions. It supports multiple providers (Gemini, OpenAI, Anthropic), context-awareness per directory, and safe command execution.

# Product Definition

## Vision
The goal of `ask` is to be the ultimate frictionless CLI companion for developers, enabling natural language interaction with AI models directly in the terminal. The core philosophy is "Enhance, don't Interrupt" — acting as a transparent layer that augments the shell experience without imposing new workflows. The immediate focus is on perfecting the existing foundation: improving performance, safety, and delivering a superior Developer Experience (DX) and User Experience (UX) through polished interactions and intuitive configuration.

## Strategic Objectives (Short-Term)
1.  **Refine & Polish Core Experience:** Prioritize the stability, responsiveness, and usability of existing features over introducing new complexities.
2.  **Optimize Performance & Safety:** Reduce latency in API interactions and strengthen the safety mechanisms for command execution (e.g., smarter destructive command detection).
3.  **Elevate UX/DX:** Simplify configuration management, improve visual feedback, and ensure the tool integrates seamlessly into daily development workflows.

## Key Features & Priorities

### 1. Optimization & Robustness (Priority)
-   **Performance Tuning:** Optimize startup time and reduce overhead in API requests.
-   **Enhanced Safety:** Refine the logic for detecting destructive commands (`rm`, `dd`, `mv`) to reduce false positives/negatives.
-   **Smarter Command Injection:** Improve the reliability of command suggestions and the "sudo retry" mechanism.

### 2. User Experience (UX) & Visual Feedback
-   **Rich Output Formatting:** Improve the rendering of markdown, code blocks, and tables in the terminal.
-   **Interactive Feedback:** Enhance spinners and progress indicators, especially for longer operations like "Thinking" mode.
-   **Clearer Errors:** Transform generic error messages into actionable guidance for the user.

### 3. Developer Experience (DX) & Configuration
-   **Intuitive Configuration:** Simplify the structure of `config.toml` and make the `ask init` process more powerful and user-friendly.
-   **Transparent Config Resolution:** Ensure absolute clarity in how configuration is merged from flags, environment variables, and config files (Global vs. Local).
-   **Shell Integration:** Deepen integration with shell completions and aliases to make usage second nature.

### 4. Workflow Integration
-   **Automation Ready:** Polish support for pipes (`|`) and non-interactive modes (`--json`) to facilitate use in scripts and CI/CD pipelines.
-   **Proactive Assistance:** Refine context-awareness to better suggest commands based on the current directory state and history.

## Success Metrics
-   **Reduction in Configuration Errors:** Fewer support issues related to setup and profile management.
-   **Increased Safety Confidence:** Users trust the tool to handle dangerous commands correctly.
-   **Usage Frequency:** Users integrate `ask` into their daily loop more frequently due to reduced friction.
