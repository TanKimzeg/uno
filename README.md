# UNO 纸牌游戏 🎮

一个用 Rust 语言实现的经典 UNO 纸牌游戏！

## 项目简介

这是一个完整的 UNO 纸牌游戏实现，包含了所有经典的游戏规则和功能：

- 🃏 完整的 UNO 牌组（数字牌、功能牌、万能牌）
- 🎯 经典游戏规则实现
- 🎨 彩色控制台输出
- 👥 多玩家支持
- 🔄 方向变换、跳过、抽牌等功能牌效果
- 🌈 万能牌颜色选择

## 功能特性

### 卡牌类型
- **数字牌**: 0-9，四种颜色（红、绿、蓝、黄）
- **功能牌**: 跳过（Skip）、反转（Reverse）、抽2张（Draw Two）
- **万能牌**: 变色牌（Wild）、万能抽4张（Wild Draw Four）

### 游戏规则
- 每位玩家初始7张牌
- 按颜色、数字或功能类型出牌
- 万能牌可以在任何时候使用
- 手牌剩余1张时需要喊"UNO"
- 最先出完所有牌的玩家获胜

## 快速开始

### 环境要求
- Rust 1.70.0 或更高版本
- Cargo 包管理器

### 安装与运行

1. **克隆项目**
   ```bash
   git clone https://github.com/TanKimzeg/uno.git
   cd uno
   ```

2. **构建项目**
   ```bash
   cargo build
   ```

3. **快速体验游戏**
   ```bash
   cargo run --example example
   ```

4. **运行测试**
   ```bash
   cargo test --show-ouput
   ```

## 项目结构

```
src/
├── lib.rs          # 库入口文件
├── cards.rs        # 卡牌定义和相关功能
├── game.rs         # 游戏逻辑和流程控制
└── player.rs       # 玩家相关功能

examples/
└── example.rs      # 游戏示例

tests/
├── cards_test.rs   # 卡牌功能测试
├── game_test.rs    # 游戏逻辑测试
└── player_test.rs  # 玩家功能测试
```

## 使用方法

### 基本用法

```rust
use uno::game::UnoGame;

fn main() {
    // 创建游戏实例
    let mut uno_game = UnoGame::new();
    
    // 设置玩家
    let players = vec!["Alice", "Bob", "Charlie"];
    
    // 开始游戏
    uno_game.play(players);
}
```

### 作为库使用

在你的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
uno = { path = "path/to/uno" }
```

然后在代码中导入：

```rust
use uno::game::UnoGame;
use uno::cards::{UnoCard, Color, Number};
use uno::player::Player;
```

## 游戏界面

游戏提供了彩色的控制台界面：
- 🔴 红色卡牌显示为红色
- 🟢 绿色卡牌显示为绿色  
- 🔵 蓝色卡牌显示为蓝色
- 🟡 黄色卡牌显示为黄色

## 技术实现

### 核心组件

1. **卡牌系统** (`cards.rs`)
   - 枚举定义所有卡牌类型
   - 实现卡牌验证逻辑
   - 提供彩色显示功能

2. **游戏引擎** (`game.rs`)
   - 游戏状态管理
   - 回合制逻辑
   - 胜负判定

3. **玩家管理** (`player.rs`)
   - 手牌管理
   - 用户输入处理
   - 策略选择

### 依赖库

- `colored`: 控制台彩色输出
- `rand`: 随机数生成（洗牌等）

## 开发与贡献

### 运行测试

```bash
# 运行所有测试
cargo test -- --show-output

# 运行特定测试文件
cargo test cards_test
cargo test game_test
cargo test player_test
```

## 版本信息

- **当前版本**: 0.1.0
- **Rust 版本**: 2021 Edition

## 许可证

本项目采用 GPLv3 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 致谢

感谢所有为这个项目做出贡献的开发者！

---

🎉 **现在就开始游戏吧！运行 `cargo run --example example` 来体验 UNO 的乐趣！**