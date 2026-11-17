# lcctl

用于 [leetcode.cn](https://leetcode.cn) 日常刷题任务的 Rust 命令行辅助工具。

## 功能概览

- `lcctl auth`：管理本地会话凭证（`LEETCODE_SESSION` / `csrftoken`）与默认语言/目录配置，校验在线登录状态。
- `lcctl daily`：查看今日每日一题、历史某月每日一题（`--month YYYY-MM`）及打卡连胜状态，支持 `--fetch` 一键拉取代码模板。
- `lcctl show <slug|id>`：查看题目中/英文题面、样例、标签和提示，支持 `--fetch` 生成本地代码模板文件。
- `lcctl list`：搜索或分页浏览题库题目（需登录）。
- `lcctl submissions <slug|id>` / `lcctl submission <id>`：查看指定题目的提交记录或某次提交的源码与运行指标（需登录）。
- `lcctl stats [userSlug]`：查看用户刷题进度、语言分布、打卡日历、最近 AC 与竞赛积分。
- `lcctl points`：查看个人积分余额与近期积分明细（需登录）。
- 全局支持 `--json` 输出和 `--cookie` 临时覆盖。

## 快速开始

### 编译
```bash
cargo build --release
```

### 交互式终端模式（类 Claude Code / AGY CLI）
不带任何子命令直接运行 `lcctl`（或显式运行 `lcctl repl`）即可进入交互式终端：

```bash
lcctl
```

交互模式特性：
- **斜杠命令与 Tab 补全**：支持 `/daily`、`/show`、`/fetch`、`/hints`、`/use`、`/list`、`/submissions`、`/submission`、`/stats`、`/points`、`/auth`、`/help`、`/clear`、`/exit`（也兼容省略 `/`）。
- **上下文记忆**：执行 `/daily` 或 `/show two-sum`（或 `/use 1`）后自动记录当前活动题目，随后直接输入 `/fetch`、`/hints` 或 `/submissions` 即可省略题目标识。
- **命令历史**：历史记录自动保存至用户数据目录 `lcctl/history.txt`，支持上下键翻阅。
- **原生滚屏**：不接管全屏缓冲区，保留终端原生滚动与鼠标复制体验。

### 配置登录凭证
推荐使用内置的浏览器交互登录（基于 `chromiumoxide` 自动拉起 Chrome/Edge 并提取登录态）：

```bash
# 打开可见浏览器窗口完成登录并自动保存 LEETCODE_SESSION / csrftoken
lcctl auth login

# 如需清理该独立浏览器目录的旧 Cookie 重新登录：
lcctl auth login --fresh

# 如需手动指定浏览器路径：
lcctl auth login --browser-path "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
```

也可以在浏览器登录 `https://leetcode.cn` 后手动复制 `LEETCODE_SESSION`（以及可选 `csrftoken`）：

```bash
# 方式 1：保存整段 Cookie 到配置文件
lcctl auth set --cookie "LEETCODE_SESSION=...; csrftoken=..."

# 方式 2：分别传入字段
lcctl auth set --session "<LEETCODE_SESSION>" --csrf "<csrftoken>"

# 方式 3：通过环境变量临时提供
export LEETCODE_SESSION="..."
export LEETCODE_CSRFTOKEN="..."

# 验证登录态
lcctl auth status
```

配置文件默认保存在系统的用户配置目录下 `lcctl/config.toml`（可通过环境变量 `LCCTL_CONFIG` 自定义路径）。

### 常用命令示例

```bash
# 查看每日一题并生成 Rust 模板到 ./problems
lcctl auth config --lang rust --dir ./problems
lcctl daily --fetch

# 查看指定题目详情（支持题号需登录，支持 slug 无需登录）
lcctl show two-sum --hints
lcctl show two-sum --fetch --lang golang

# 搜索题库（需登录）
lcctl list --keyword "two" --limit 10

# 查看提交记录与单次提交源码（需登录）
lcctl submissions two-sum --limit 5
lcctl submission 607391099

# 查看刷题统计（省略用户名时查询当前登录用户）
lcctl stats huxulm

# 查看个人积分余额与近期明细（需登录）
lcctl points --limit 15
```

## API 文档
详见 [docs/api-reference.md](docs/api-reference.md)。
