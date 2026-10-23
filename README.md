# mongol-unicode-renderer

**Unicode 蒙文塑形、竖排换列与无接缝渲染。独立 Rust 库，无游戏引擎依赖。**

[English](README.en.md) · [API / design](docs/design.md) · [Font notices](THIRD_PARTY.md)

从游戏的 Unicode 蒙文显示部分拆出的独立项目，重点是**正确塑形与连续笔画**，
不是把整段文字简单旋转，也不是另造一个字体。默认使用平滑渲染，像素风是可选项。

![Noto Sans Mongolian rendered specimen](docs/demo.png)

上图由本项目实际生成，使用随测试附带的开源 Noto Sans Mongolian；与游戏画面无关。

## 已实现

- HarfRust / OpenType 上下文塑形，处理词首、词中、词尾等连接形式。
- Unicode 原文传给塑形器：不剥除 FVS1–4、MVS、ZWJ / ZWNJ；具体字形取决于字体。
- 从上到下书写、列从左向右排列，使用不镜像的蒙文坐标变换。
- 用实际字形轮廓测量并按词换列，不按字符数量硬切。
- U+202F 窄不换行空格和 U+00A0 不换行空格不会成为换列点，后缀组保持完整。
- 一整列的字形轮廓合并后栅格化，避免旋转逐字图集时出现的横向接缝。
- 平滑模式支持 1–4 倍整体超采样；像素模式不抗锯齿、不缩放。
- 输出 **straight-alpha RGBA8**，方便接入 Bevy、其他游戏引擎或图片编码器。
- 缺字、私用区输入、过长单词、无效参数和超大图片显式报错。
- 原生测试、PNG 示例、Windows / Linux / WASM 编译 CI。

## 快速运行

需要 Rust 1.85+。仓库带有一个有明确 OFL 许可的测试字体，不依赖系统字体。

```sh
git clone https://github.com/romchina/mongol-unicode-renderer.git
cd mongol-unicode-renderer
cargo run --locked --example render -- tests/fonts/NotoSansMongolian-Regular.ttf output/demo.png
```

指定自己的 Unicode 文本与字体：

```sh
cargo run --locked --example render -- path/to/font.ttf output/custom.png --text "ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ" --size 32 --height 256
cargo run --locked --example render -- path/to/font.ttf output/file.png --text-file text.txt
```

像素字体可加 `--pixel`，并选用该字体设计网格的整数倍字号。此选项不会让普通字体
变成设计良好的像素字体，也不会自动修正浏览器或引擎的非整数缩放。

PNG 示例默认白底，`--transparent` 保留透明通道；核心库始终返回透明 RGBA。
README 的样图使用默认示例文字，加 `--size 48 --height 240` 生成。

## 作为库使用

尚未发布到 crates.io，可使用本地 path，或固定 Git commit（不要依赖移动分支）：

```toml
[dependencies]
mongol-unicode-renderer = { path = "../mongol-unicode-renderer" }
```

```rust
use mongol_unicode_renderer::{Font, LayoutOptions, RasterOptions, render_layout};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("tests/fonts/NotoSansMongolian-Regular.ttf")?;
let font = Font::new(&bytes)?;
let layout = font.layout("ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ", &LayoutOptions {
    font_size: 32.0,
    max_height: 256.0,
    ..Default::default()
})?;
let image = render_layout(&font, &layout, &RasterOptions::default())?;
// image.width / image.height / image.rgba -> 纹理或 PNG
Ok(())
}
```

## 范围和限制

- **仅 Unicode 路径**，不是 Menksoft 私用区到 Unicode 的转换器。
- 输入应是逻辑顺序的蒙文，配合含蒙文 OpenType 替换规则的轮廓字体；读取 face 0。
- 不是完整的多语种段落 / 双向文本 / CJK 竖排排版引擎。混合脚本应先分段，
  字体回退、直立拉丁标点、字体集合选面、可变轴实例等留给上层实现。
- 常规空格只在 U+0020 换列，重复与首尾常规空格被归一化；CRLF/CR 归一化为 LF。
  不声称实现完整 Unicode Line Breaking Algorithm。选择符不被拆开不代表每个字体
  都支持每个选择序列。
- 过长的整词默认返回 `WordTooTall`。可显式使用 `OverflowPolicy::Keep` 保留原字号、
  返回溢出标记；从不偷偷缩小字体、裁切或破坏连写。
- 位图、彩色 / SVG 字形以及高级字体特性配置不在当前范围内。
- 输入上限 64 KiB；图片单边最多 8192px、单次栅格/画布最多 16,777,216 像素
  （含超采样）。这些是防误用护栏，不是对恶意输入的完整资源隔离。
- WASM CI 仅检查核心库可编译；当前未提供 JS 绑定或网页编辑器。

## 开发与测试

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
rustup target add wasm32-unknown-unknown
cargo check --locked --lib --target wasm32-unknown-unknown
```

测试涵盖连接字形、选择符/连接符、NNBSP、按词换列、显式溢出、缺字、文字矩阵方向、
无接缝、边界留白、透明通道和分配上限；这不是完整蒙文正字法的一致性认证。
欢迎用可公开的 Unicode 最小复现、字体来源和截图提交问题。

## 许可与公开边界

- **项目代码：MIT**，见 [LICENSE](LICENSE)。
- **测试字体：SIL OFL 1.1**，见 [tests/fonts/OFL.txt](tests/fonts/OFL.txt)，不属于 MIT。
- 库不嵌入字体；集成者须自行确认字体的使用、嵌入和分发权限。
- 本仓库不包含游戏素材、剧本、商业试用字体、私用区字库、凭据或部署配置。
