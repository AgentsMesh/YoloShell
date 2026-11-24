---
name: arch-check
description: >-
  架构与实现审查 —— 基于「概念建模 → 职责划分 → 机制/策略分离 → 因果与不变量 → 属性建模 → 模块化 → SOLID → GRASP → YAGNI」的全维度审查，带置信度门控与假阳性抑制（对抗"过度工程建议"这类 AI slop）。触发于：要求 review/审查架构、检查目录结构/依赖关系/职责划分、重构前评估、技术债盘点、判断是否过度设计，或问「这个设计合理吗 / 该怎么拆 / 有没有循环依赖 / 这个改动架构上 OK 吗」。一律按最高强度审查。用法：`arch-check [范围] [--fix|--plan]`。范围可为目录/文件/PR；缺省审当前分支相对基线的全部变更。
argument-hint: "[范围] [--fix|--plan]"
---

# Claude Code Adapter

1. 动手前先完整读 `shared/SKILL.md`，之后**按需**加载当前 lens 对应的
   `shared/references/` 文件——不要一次性把 reference 全读进上下文。配套 reference
   相对 `shared/` 解析，`AGENTS.md`、`docs/` 等仓库路径相对 checkout 根目录解析。
2. 把 `$ARGUMENTS` 解析成 `[范围] [--fix|--plan]`，其余按共享工作流。
3. 步骤 1「采集上下文」里，宿主的用户级约定文件是 `~/.claude/CLAUDE.md`；和仓库根
   的 `CLAUDE.md` / `AGENTS.md`、以及变更文件祖先目录里的同名文件一起钉住。
   Yolo 的真实组件边界、PTY custody、terminal/agent 生命周期与通信契约
   **压过任何通用理想架构**；结合 `docs/architecture.md` 和相关设计文档取证。
4. 用 **`Agent`** 工具做 fan-out：步骤 3 每个 lens（A–E）一个 agent，步骤 4 每条
   finding 一个独立复核 agent。同一阶段按宿主容量分批并发，每个 lens 保留独立上下文。
   本上下文拿不到 `Agent` 时，走共享工作流的单遍降级路径，并在总结里说明。
5. `--plan` 用 **`TaskCreate`** 落成可勾选清单，按 Phase 分组、标 S/M/L 工作量。
6. `--fix` 只改 🟢/🟡 里低风险、机械、不动公共契约的项；🔴 概念/职责级重构只出方案
   不自动改。改完按 `AGENTS.md` 的环境与测试说明跑相关
   Bazel target：共享 Rust/runtime 在 Ubuntu，macOS 专属验证在 macOS；
   保留现有 PTY，自动化 TUI 验收使用私有 PTY，不抢窗口焦点。

Claude Code 另有内置 `/code-review`（含云端 `ultra` 模式），那是 diff 级正确性审查；
本 skill 审的是架构维度，两者互补。共享工作流是 Codex 与 Claude Code 共用的同一份，
改逻辑只改 `shared/`。
