# UNO 纸牌游戏 🎮

一个用 Rust 语言实现的经典 UNO 纸牌游戏！(正在开发中)

## 功能概览

- 核心 UNO 规则：颜色/数字/功能牌匹配，反转、跳过、抽2、万能、万能+4 等
- 多玩家对战（通过本地 TCP 连接）
- 事件驱动的游戏引擎与协议（JSON Lines）
- 两种形态
  - 服务端：`server`
  - 终端 TUI 客户端：`tui_client`

## 运行环境

- Rust 1.70+（2021 Edition）
- Windows、macOS、Linux 均可
- 默认服务监听/客户端连接地址：`127.0.0.1:9000`

## 快速体验

- 构建
  - `cargo build`
- 启动服务端（窗口1）
  - `cargo run --bin server`
- 启动 TUI 客户端（窗口2）
  - `cargo run --bin tui_client`

提示：第一次运行会下载构建依赖，时间略长。

## TUI 客户端操作说明

连接后在 TUI 窗口按键操作：

- J：加入游戏（昵称默认取环境变量 USERNAME/USER）
- S：开始游戏（当前玩家为房主时可触发）
- ←/→：移动手牌光标
- Enter：出牌
  - 当选择“万能/万能+4”且未指定颜色时，按下 Enter 后使用 R/G/B/Y 选择颜色
- D：摸牌
- P：跳过
- U：出牌并同时喊 UNO（当手牌数为 2 且准备出第 2 张时）
- Q 或 Esc：退出客户端
- 颜色选择（当需要选择时）：
  - R = 红，G = 绿，B = 蓝，Y = 黄

界面区域：
- 左侧：玩家与各自剩余手牌数（“←”指向当前回合）
- 中间：桌面状态（顶牌与操作提示）
- 右侧：你的手牌（“[]”包裹的是当前光标所选）
- 底部：最近日志

## 项目结构

```
src/
├── lib.rs
├── bin/
│   ├── client.rs        # 命令行客户端入口
│   ├── server.rs        # 服务端入口
│   └── tui_client.rs    # TUI 客户端入口（ratatui/crossterm）
├── game/
│   ├── cards.rs         # 卡牌与颜色/点数/功能定义
│   ├── events.rs        # 游戏事件（引擎 -> 客户端）
│   ├── mod.rs
│   ├── player.rs        # 玩家与手牌逻辑
│   └── uno_game.rs      # 游戏引擎
├── ports/
│   ├── bus.rs           # 事件总线/消息端口
│   └── mod.rs
└── protocol/
    ├── client2server.rs # 客户端 -> 服务端 协议（JSON）
    ├── mod.rs
    └── server2client.rs # 服务端 -> 客户端 协议（JSON）

tests/
├── bus_test.rs
├── cards_test.rs
├── game_test.rs
└── player_test.rs
```

## 协议与通信

- 传输：基于 TCP 的 JSON Lines（每条消息一行 JSON）
- 客户端写入：`serde_json::to_string` + `writeln!()`
- 服务端读取：逐行解析为 `protocol::*` 枚举
- 重要事件：`game/events.rs` 中的 `GameEvent`（如 PlayerTurn、CardPlayed、TopCardChanged 等）

## 测试

- 运行全部测试
  - `cargo test -- --show-output`
- 运行指定测试模块
  - `cargo test cards_test`
  - `cargo test game_test`
  - `cargo test player_test`

## 依赖

- UI：`ratatui`、`crossterm`
- 并发/通道：`flume`
- 序列化：`serde`、`serde_json`
- 随机：`rand`


## 贡献

欢迎 Issue / PR：

1. Fork 本仓库
2. 创建分支：`git checkout -b feature/your-feature`
3. 提交：`git commit -m "feat: your feature"`
4. 推送：`git push origin feature/your-feature`
5. 发起 Pull Request

## 许可证

本项目采用 GPLv3 许可证，详见 [LICENSE](./LICENSE)。
